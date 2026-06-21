import AppKit

@MainActor
final class LatencyHUD: NSTextField {
    init() {
        super.init(frame: .zero)
        stringValue = "p95 -- ms"
        isEditable = false
        isBordered = false
        drawsBackground = true
        backgroundColor = NSColor.black.withAlphaComponent(0.18)
        textColor = .secondaryLabelColor
        font = .monospacedSystemFont(ofSize: 11, weight: .medium)
        alignment = .center
        wantsLayer = true
        layer?.cornerRadius = 8
        layer?.cornerCurve = .continuous
        translatesAutoresizingMaskIntoConstraints = false
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func update(p95Milliseconds: Double?) {
        if let p95Milliseconds {
            stringValue = "p95 \(Int(p95Milliseconds)) ms"
        } else {
            stringValue = "p95 -- ms"
        }
    }
}
