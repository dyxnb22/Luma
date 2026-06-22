import AppKit
import LumaCore

@MainActor
final class ActivitySparklineView: NSView {
    private var buckets: [PersistentUsageTracker.ActivityBucket] = []

    override var isFlipped: Bool { true }

    func apply(buckets: [PersistentUsageTracker.ActivityBucket]) {
        self.buckets = buckets
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard !buckets.isEmpty else { return }
        let maxCount = max(buckets.map(\.count).max() ?? 1, 1)
        let barWidth = bounds.width / CGFloat(buckets.count)
        let gap: CGFloat = 2
        NSColor.controlAccentColor.withAlphaComponent(0.75).setFill()
        for (index, bucket) in buckets.enumerated() {
            let height = CGFloat(bucket.count) / CGFloat(maxCount) * (bounds.height - 14)
            let rect = NSRect(
                x: CGFloat(index) * barWidth + gap,
                y: bounds.height - height - 2,
                width: max(1, barWidth - gap * 2),
                height: max(2, height)
            )
            rect.fill()
        }
    }
}

@MainActor
final class ActivitySettingsView: NSView {
    private let usage: PersistentUsageTracker
    private let sparkline7 = ActivitySparklineView()
    private let sparkline30 = ActivitySparklineView()
    private let moduleSummaryLabel = NSTextField(wrappingLabelWithString: "")

    init(usage: PersistentUsageTracker) {
        self.usage = usage
        super.init(frame: .zero)
        setup()
        refresh()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func refresh() {
        Task {
            let buckets7 = await usage.activityBuckets(lastDays: 7)
            let buckets30 = await usage.activityBuckets(lastDays: 30)
            let since30 = Calendar.current.date(byAdding: .day, value: -30, to: Date()) ?? Date()
            let byModule = await usage.countsByModule(since: since30)
            let summary = byModule.sorted { $0.value > $1.value }.prefix(8).map { "\($0.key.rawValue): \($0.value)" }.joined(separator: " · ")
            await MainActor.run {
                sparkline7.apply(buckets: buckets7)
                sparkline30.apply(buckets: buckets30)
                moduleSummaryLabel.stringValue = summary.isEmpty ? "No usage recorded yet." : summary
            }
        }
    }

    private func setup() {
        translatesAutoresizingMaskIntoConstraints = false
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12
        stack.translatesAutoresizingMaskIntoConstraints = false

        stack.addArrangedSubview(section("Last 7 days"))
        sparkline7.translatesAutoresizingMaskIntoConstraints = false
        sparkline7.heightAnchor.constraint(equalToConstant: 48).isActive = true
        sparkline7.widthAnchor.constraint(equalToConstant: 420).isActive = true
        stack.addArrangedSubview(sparkline7)

        stack.addArrangedSubview(section("Last 30 days"))
        sparkline30.translatesAutoresizingMaskIntoConstraints = false
        sparkline30.heightAnchor.constraint(equalToConstant: 48).isActive = true
        sparkline30.widthAnchor.constraint(equalToConstant: 420).isActive = true
        stack.addArrangedSubview(sparkline30)

        stack.addArrangedSubview(section("By module (30 d)"))
        moduleSummaryLabel.font = TypographyTokens.caption()
        moduleSummaryLabel.textColor = .secondaryLabelColor
        stack.addArrangedSubview(moduleSummaryLabel)

        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            stack.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -12),
            stack.bottomAnchor.constraint(lessThanOrEqualTo: bottomAnchor, constant: -12)
        ])
    }

    private func section(_ title: String) -> NSTextField {
        let label = NSTextField(labelWithString: title)
        label.font = TypographyTokens.title3
        return label
    }
}
