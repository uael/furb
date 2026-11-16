// The voice of the operator as text, on Whisper large-v3 turbo through WhisperKit, on the Neural
// Engine of this Mac, with no network once its model stands, for the window of furb. The window
// starts it with its first frame and gives it the folder of its models. A first start downloads
// the model there, as {"loading":"download","done":42}, then prepares it for this Mac, as
// {"loading":"prepare"}, which takes minutes once and seconds after; then it readies the
// microphone with no sound in and says {"ready":true}. Each line "start" on its input opens the
// microphone and starts a record, and each line "stop" ends it. During a record it writes one line
// of JSON for each change: the level of the microphone from 0 to 100, as {"level":12}, and the whole
// text so far, as {"text":"..."}, then the final text as {"text":"...","final":true}. Whisper finds
// the language of each record by itself. A fault writes {"error":"..."}. It ends once its input ends.
import AVFoundation
import Foundation
import WhisperKit

/// The model: Whisper large-v3 turbo, quantized for the Neural Engine to 632 MB.
let variant = "openai_whisper-large-v3-v20240930_turbo_632MB"

/// The samples of a second of the audio that Whisper reads.
let second = 16000

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

/// The model loaded once, then a record for each "start" until the input ends.
func serve() async throws {
  guard CommandLine.arguments.count > 1 else {
    throw Fault(errorDescription: "The window gave the speech no folder for its model.")
  }
  let folder = try await model(in: URL(fileURLWithPath: CommandLine.arguments[1]))
  told(["loading": "prepare"])
  let config = WhisperKitConfig(
    modelFolder: folder.path, verbose: false, logLevel: .none, prewarm: true, load: true, download: false)
  let kit = try await WhisperKit(config)
  let microphone = try Microphone()
  told(["ready": true])
  var guessing: Task<Void, Never>?
  for await line in commands() {
    switch line {
    case "start" where guessing == nil:
      do {
        try microphone.open()
      } catch {
        told(["error": error.localizedDescription])
        continue
      }
      guessing = Task { await guessed(kit, microphone) }
    case "stop":
      guard let guess = guessing else { continue }
      guessing = nil
      let samples = microphone.close()
      guess.cancel()
      await guess.value
      do {
        told(["text": try await text(kit, samples, microphone.spoke), "final": true])
      } catch {
        told(["error": error.localizedDescription])
      }
    default:
      continue
    }
  }
}

/// The folder of the model in the folder of the models, downloaded there once.
func model(in base: URL) async throws -> URL {
  let local = base.appending(path: "models/argmaxinc/whisperkit-coreml/\(variant)")
  let parts = ["AudioEncoder.mlmodelc", "TextDecoder.mlmodelc", "MelSpectrogram.mlmodelc"]
  if parts.allSatisfy({ FileManager.default.fileExists(atPath: local.appending(path: $0).path) }) {
    return local
  }
  let said = Said()
  do {
    return try await WhisperKit.download(variant: variant, downloadBase: base) { progress in
      said.once(Int(progress.fractionCompleted * 100))
    }
  } catch {
    throw Fault(errorDescription: "The model of the speech did not download: \(error.localizedDescription)")
  }
}

/// The share of the download that the window was told last, so each share goes once.
final class Said: @unchecked Sendable {
  let lock = NSLock()
  var last = -1

  func once(_ done: Int) {
    lock.lock()
    defer { lock.unlock() }
    guard done != last else { return }
    last = done
    told(["loading": "download", "done": done])
  }
}

/// The text so far of the record that runs, read again each time a quarter of a second of new
/// sound came, until the record ends. A guess that fails says nothing: the final text says why.
func guessed(_ kit: WhisperKit, _ microphone: Microphone) async {
  var read = 0
  while !Task.isCancelled {
    let samples = microphone.samples()
    guard samples.count - read >= second / 4 else {
      try? await Task.sleep(for: .milliseconds(50))
      continue
    }
    read = samples.count
    if let said = try? await text(kit, samples, microphone.spoke), !said.isEmpty, !Task.isCancelled {
      told(["text": said])
    }
  }
}

/// What Whisper reads in the samples of a record, in the language that it finds. A record that
/// never rose above a murmur holds no words, where Whisper would read some in its silence.
func text(_ kit: WhisperKit, _ samples: [Float], _ spoke: Bool) async throws -> String {
  guard spoke, samples.count >= second / 4 else { return "" }
  let options = DecodingOptions(
    task: .transcribe, language: nil, detectLanguage: true, skipSpecialTokens: true, withoutTimestamps: true)
  let results = try await kit.transcribe(audioArray: samples, decodeOptions: options)
  return results.map(\.text).joined().trimmingCharacters(in: .whitespacesAndNewlines)
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

/// The microphone, prepared with no sound in, so a record that starts only starts its engine. Its
/// tap keeps each buffer of a record in the format that Whisper reads, 16 kHz in one channel, and
/// says the level of the sound about each 50 ms.
final class Microphone: @unchecked Sendable {
  let engine = AVAudioEngine()
  let lock = NSLock()
  var recording = false
  var kept: [Float] = []
  var loudest = 0
  var spoke = false
  var said = Date.distantPast

  init() throws {
    let node = engine.inputNode
    let hardware = node.outputFormat(forBus: 0)
    guard
      let format = AVAudioFormat(
        commonFormat: .pcmFormatFloat32, sampleRate: Double(second), channels: 1, interleaved: false),
      let converter = AVAudioConverter(from: hardware, to: format)
    else {
      throw Fault(errorDescription: "The audio of the microphone cannot be converted.")
    }
    node.installTap(onBus: 0, bufferSize: 1024, format: hardware) { [self] buffer, _ in
      lock.lock()
      defer { lock.unlock() }
      guard recording else { return }
      let now = level(of: buffer)
      loudest = max(loudest, now)
      spoke = spoke || now >= 5
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
      if let channel = converted.floatChannelData?[0] {
        kept.append(contentsOf: UnsafeBufferPointer(start: channel, count: Int(converted.frameLength)))
      }
    }
    engine.prepare()
  }

  /// The sound into a new record, from now on.
  func open() throws {
    lock.lock()
    (recording, kept, spoke) = (true, [], false)
    lock.unlock()
    try engine.start()
  }

  /// The samples of the record so far.
  func samples() -> [Float] {
    lock.lock()
    defer { lock.unlock() }
    return kept
  }

  /// The microphone closed, and the samples of the record that it ends.
  func close() -> [Float] {
    engine.pause()
    lock.lock()
    defer { lock.unlock() }
    recording = false
    return kept
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
