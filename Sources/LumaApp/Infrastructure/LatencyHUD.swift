import Foundation
import OSLog
import AppKit
import LumaCore

@MainActor
final class LatencyTelemetry {
    static let shared = LatencyTelemetry()

    private let logger = Logger(subsystem: "app.luma", category: "latency")
    private var hotkeySamples: [Double] = []
    private var keystrokeSamples: [Double] = []
    private let capacity = 200

    static func report(p95Milliseconds: Double) {
        reportHotkey(p95Milliseconds)
    }

    static func reportHotkey(_ milliseconds: Double) {
        shared.record(milliseconds, into: \.hotkeySamples)
    }

    static func reportKeystroke(_ milliseconds: Double) {
        shared.record(milliseconds, into: \.keystrokeSamples)
    }

    func record(_ ms: Double, into keyPath: WritableKeyPath<LatencyTelemetry, [Double]>) {
        switch keyPath {
        case \.hotkeySamples:
            hotkeySamples.append(ms)
            if hotkeySamples.count > capacity {
                hotkeySamples.removeFirst(hotkeySamples.count - capacity)
            }
        case \.keystrokeSamples:
            keystrokeSamples.append(ms)
            if keystrokeSamples.count > capacity {
                keystrokeSamples.removeFirst(keystrokeSamples.count - capacity)
            }
        default:
            break
        }
        #if DEBUG
        if (hotkeySamples.count + keystrokeSamples.count) % 20 == 0 {
            logger.debug("Rolling hotkey p95: \(Int(self.p95(for: self.hotkeySamples)))ms keystroke p95: \(Int(self.p95(for: self.keystrokeSamples)))ms")
        }
        #endif
    }

    func currentP95() -> Double {
        let combined = hotkeySamples + keystrokeSamples
        return p95(for: combined)
    }

    func hotkeyP95() -> Double {
        p95(for: hotkeySamples)
    }

    func keystrokeP95() -> Double {
        p95(for: keystrokeSamples)
    }

    func recentSamples() -> [Double] {
        hotkeySamples + keystrokeSamples
    }

    struct ExportReport: Codable {
        let generatedAt: String
        let hotkeyP95Milliseconds: Double
        let keystrokeP95Milliseconds: Double
        let combinedP95Milliseconds: Double
        let hotkeySampleCount: Int
        let keystrokeSampleCount: Int
        let hotkeySamples: [Double]
        let keystrokeSamples: [Double]
    }

    @discardableResult
    func exportReport() throws -> URL {
        let directory = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Logs/Luma", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("latency-report.json")
        let report = ExportReport(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            hotkeyP95Milliseconds: hotkeyP95(),
            keystrokeP95Milliseconds: keystrokeP95(),
            combinedP95Milliseconds: currentP95(),
            hotkeySampleCount: hotkeySamples.count,
            keystrokeSampleCount: keystrokeSamples.count,
            hotkeySamples: hotkeySamples,
            keystrokeSamples: keystrokeSamples
        )
        let data = try JSONEncoder().encode(report)
        try data.write(to: url, options: .atomic)
        return url
    }

    private func p95(for samples: [Double]) -> Double {
        guard !samples.isEmpty else { return 0 }
        let sorted = samples.sorted()
        let idx = Int(Double(sorted.count - 1) * 0.95)
        return sorted[min(idx, sorted.count - 1)]
    }
}

@MainActor
final class LatencyHUDOverlayView: NSView {
    private let label = NSTextField(labelWithString: "")

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.withAlphaComponent(0.55).cgColor
        layer?.cornerRadius = 6
        label.font = TypographyTokens.caption(weight: .medium)
        label.textColor = .white
        label.alignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: topAnchor, constant: 4),
            label.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -4),
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            label.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8)
        ])
        refresh()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func refresh() {
        let hotkey = Int(LatencyTelemetry.shared.hotkeyP95())
        let keystroke = Int(LatencyTelemetry.shared.keystrokeP95())
        let count = LatencyTelemetry.shared.recentSamples().count
        label.stringValue = "hk \(hotkey) ms · ks \(keystroke) ms · n=\(count)"
    }
}
