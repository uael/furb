// The voice of the operator as text, on the speech of this Mac (SpeechAnalyzer, macOS 26), for the
// window of furb, which starts it while he holds the space bar. It writes one line of JSON for each
// change: the level of the microphone from 0 to 100, as {"level":12}, and the whole text so far
// with the words that may still change, as {"text":"..."}. It ends once its input ends, with the
// final text, as {"text":"...","final":true}, or with {"error":"..."} when it cannot listen.
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
      try await listen()
    } catch {
      told(["error": error.localizedDescription])
      exit(1)
    }
  }
}

func listen() async throws {
  let wanted = ProcessInfo.processInfo.environment["FURB_VOICE_LOCALE"].map(Locale.init) ?? .current
  guard let locale = await SpeechTranscriber.supportedLocale(equivalentTo: wanted) else {
    throw Fault(errorDescription: "This Mac has no speech for \(wanted.identifier).")
  }
  let transcriber = SpeechTranscriber(
    locale: locale, transcriptionOptions: [], reportingOptions: [.volatileResults], attributeOptions: [])
  try await AssetInventory.reserve(locale: locale)
  if let install = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
    try await install.downloadAndInstall()
  }
  let analyzer = SpeechAnalyzer(modules: [transcriber])
  let heard = Task {
    var final = ""
    for try await result in transcriber.results {
      let text = String(result.text.characters)
      if result.isFinal {
        final += text
      }
      told(["text": result.isFinal ? final : final + text])
    }
    return final
  }
  try await microphone(analyzer: analyzer, transcriber: transcriber)
  told(["text": try await heard.value, "final": true])
}

/// The microphone into the analyzer, in the format that it takes, until the input of this process
/// ends: the window ends the record so.
func microphone(analyzer: SpeechAnalyzer, transcriber: SpeechTranscriber) async throws {
  guard let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]) else {
    throw Fault(errorDescription: "The speech of this Mac takes no audio format.")
  }
  let (inputs, feed) = AsyncStream<AnalyzerInput>.makeStream()
  let engine = AVAudioEngine()
  let node = engine.inputNode
  let hardware = node.outputFormat(forBus: 0)
  guard let converter = AVAudioConverter(from: hardware, to: format) else {
    throw Fault(errorDescription: "The audio of the microphone cannot be converted.")
  }
  node.installTap(onBus: 0, bufferSize: 1024, format: hardware) { buffer, _ in
    told(["level": level(of: buffer)])
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
  try engine.start()
  try await analyzer.start(inputSequence: inputs)
  await Task.detached { _ = FileHandle.standardInput.readDataToEndOfFile() }.value
  engine.stop()
  node.removeTap(onBus: 0)
  feed.finish()
  try await analyzer.finalizeAndFinishThroughEndOfInput()
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
