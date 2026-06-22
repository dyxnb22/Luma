import AVFoundation
import Foundation

/// British-English text-to-speech used by the Wordbook review panel.
/// Wraps `AVSpeechSynthesizer` so callers don't have to manage retention or actor isolation.
@MainActor
public final class SpeechService {
    public static let shared = SpeechService()

    private let synthesizer = AVSpeechSynthesizer()
    private let voice: AVSpeechSynthesisVoice?

    public init(languageCode: String = "en-GB") {
        // Prefer an enhanced/premium voice for the requested locale when one is installed.
        let candidates = AVSpeechSynthesisVoice.speechVoices().filter { $0.language == languageCode }
        let preferred = candidates.first { $0.quality == .premium }
            ?? candidates.first { $0.quality == .enhanced }
            ?? candidates.first
        self.voice = preferred ?? AVSpeechSynthesisVoice(language: languageCode)
    }

    public func speak(_ text: String, rate: Float = AVSpeechUtteranceDefaultSpeechRate) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        if synthesizer.isSpeaking { synthesizer.stopSpeaking(at: .immediate) }
        let utterance = AVSpeechUtterance(string: trimmed)
        utterance.voice = voice
        utterance.rate = rate
        synthesizer.speak(utterance)
    }

    public func stop() {
        if synthesizer.isSpeaking {
            synthesizer.stopSpeaking(at: .immediate)
        }
    }
}
