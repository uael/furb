// The voice of the operator as text, on the speech of this Mac (SpeechAnalyzer, macOS 26), for the
// window of furb. The window starts it with its first frame, and it readies the speech with no
// microphone, then says {"ready":true}. Each line "start" on its input opens the microphone and
// starts a record, and each line "stop" ends it. During a record it writes one line of JSON for
// each change: the level of the microphone from 0 to 100, as {"level":12}, and the whole text so
// far with the words that may still change, as {"text":"..."}, then the final text as
// {"text":"...","final":true}. A fault writes {"error":"..."}. It ends once its input ends.
// FURB_VOICE_LOCALE names the language, the language of the system when unsaid.
import AVFoundation
import Foundation
import Speech

func told(_ line: [String: Any]) {
  guard let data = try? JSONSerialization.data(withJSONObject: line) else { return }
  FileHandle.standardOutput.write(data + Data([10]))
}

struct Fault: LocalizedError {
  let errorDescription: String?
}

@main
struct Listen {
  static func main() async {
    do {
      try await serve()
    } catch {
      told(["error": error.localizedDescription])
      exit(1)
    }
  }
}

/// The speech readied once, then a record for each "start" until the input ends.
func serve() async throws {
  let wanted = ProcessInfo.processInfo.environment["FURB_VOICE_LOCALE"].map(Locale.init) ?? .current
  guard let locale = await SpeechTranscriber.supportedLocale(equivalentTo: wanted) else {
    throw Fault(errorDescription: "This Mac has no speech for \(wanted.identifier).")
  }
  try await AssetInventory.reserve(locale: locale)
  var next = try await Readied(locale: locale)
  let microphone = try Microphone(format: next.format)
  told(["ready": true])
  var heard: Task<String, Error>?
  for await line in commands() {
    switch line {
    case "start" where heard == nil:
      let readied = next
      let (inputs, feed) = AsyncStream<AnalyzerInput>.makeStream()
      heard = Task {
        try await readied.analyzer.start(inputSequence: inputs)
        var final = ""
        for try await result in readied.transcriber.results {
          let text = String(result.text.characters)
          if result.isFinal {
            final += text
          }
          told(["text": result.isFinal ? final : final + text])
        }
        return final
      }
      do {
        try microphone.open(feed)
      } catch {
        told(["error": error.localizedDescription])
      }
    case "stop":
      guard let ending = heard else { continue }
      heard = nil
      microphone.close()
      try await next.analyzer.finalizeAndFinishThroughEndOfInput()
      told(["text": try await ending.value, "final": true])
      next = try await Readied(locale: locale)
    default:
      continue
    }
  }
}

/// The lines of the input, read on a thread of their own.
func commands() -> AsyncStream<String> {
  AsyncStream { lines in
    Thread.detachNewThread {
      while let line = readLine() {
        lines.yield(line)
      }
      lines.finish()
    }
  }
}

/// A transcriber and its analyzer, prepared for the format that they take.
struct Readied {
  let transcriber: SpeechTranscriber
  let analyzer: SpeechAnalyzer
  let format: AVAudioFormat

  init(locale: Locale) async throws {
    transcriber = SpeechTranscriber(
      locale: locale, transcriptionOptions: [], reportingOptions: [.volatileResults], attributeOptions: [])
    if let install = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
      try await install.downloadAndInstall()
    }
    guard let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]) else {
      throw Fault(errorDescription: "The speech of this Mac takes no audio format.")
    }
    self.format = format
    analyzer = SpeechAnalyzer(modules: [transcriber])
    try await analyzer.prepareToAnalyze(in: format)
  }
}

/// The microphone, prepared with no sound in, so a record that starts only starts its engine, in
/// about 150 ms. Its tap sends each buffer, in the format of the speech, to the record that runs,
/// and the level of the sound about each 50 ms.
final class Microphone: @unchecked Sendable {
  let engine = AVAudioEngine()
  let lock = NSLock()
  var feed: AsyncStream<AnalyzerInput>.Continuation?
  var loudest = 0
  var said = Date.distantPast

  init(format: AVAudioFormat) throws {
    let node = engine.inputNode
    let hardware = node.outputFormat(forBus: 0)
    guard let converter = AVAudioConverter(from: hardware, to: format) else {
      throw Fault(errorDescription: "The audio of the microphone cannot be converted.")
    }
    node.installTap(onBus: 0, bufferSize: 256, format: hardware) { [self] buffer, _ in
      lock.lock()
      defer { lock.unlock() }
      guard let feed else { return }
      loudest = max(loudest, level(of: buffer))
      if Date().timeIntervalSince(said) >= 0.05 {
        told(["level": loudest])
        (loudest, said) = (0, Date())
      }
      let frames = AVAudioFrameCount(Double(buffer.frameLength) * format.sampleRate / hardware.sampleRate) + 1
      guard let converted = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frames) else { return }
      var given = false
      converter.convert(to: converted, error: nil) { _, status in
        status.pointee = given ? .noDataNow : .haveData
        defer { given = true }
        return given ? nil : buffer
      }
      feed.yield(AnalyzerInput(buffer: converted))
    }
    engine.prepare()
  }

  /// The sound into a record, from now on.
  func open(_ feed: AsyncStream<AnalyzerInput>.Continuation) throws {
    lock.lock()
    self.feed = feed
    lock.unlock()
    try engine.start()
  }

  /// The microphone closed, and the record given no more sound.
  func close() {
    engine.pause()
    lock.lock()
    feed?.finish()
    feed = nil
    lock.unlock()
  }
}

/// The loudness of a buffer, from 0 to 100: its root mean square, which speech near the microphone
/// takes to about an eighth of the full scale.
func level(of buffer: AVAudioPCMBuffer) -> Int {
  guard let samples = buffer.floatChannelData?[0], buffer.frameLength > 0 else { return 0 }
  let count = Int(buffer.frameLength)
  var sum: Float = 0
  for at in 0..<count { sum += samples[at] * samples[at] }
  return Int(min(1, (sum / Float(count)).squareRoot() * 8) * 100)
}
