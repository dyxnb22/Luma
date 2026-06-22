import Foundation
import OSLog
import AppKit
import LumaCore

@MainActor
final class LatencyTelemetry {
    static let shared = LatencyTelemetry()
    private let logger = Logger(subsystem: "app.luma", category: "latency")
    private var samples: [Double] = []
    private let capacity = 200

    static func report(p95Milliseconds: Double) {
        shared.record(p95Milliseconds)
    }

    func record(_ ms: Double) {
        samples.append(ms)
        if samples.count > capacity { samples.removeFirst(samples.count - capacity) }
        #if DEBUG
        if samples.count % 20 == 0 {
            logger.debug("Rolling p95: \(Int(self.currentP95()))ms over \(self.samples.count) samples")
        }
        #endif
    }

    func currentP95() -> Double {
        guard !samples.isEmpty else { return 0 }
        let sorted = samples.sorted()
        let idx = Int(Double(sorted.count) * 0.95)
        return sorted[min(idx, sorted.count - 1)]
    }

    func recentSamples() -> [Double] {
        samples
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
        let p95 = Int(LatencyTelemetry.shared.currentP95())
        let count = LatencyTelemetry.shared.recentSamples().count
        label.stringValue = "p95 \(p95) ms · n=\(count)"
    }
}
