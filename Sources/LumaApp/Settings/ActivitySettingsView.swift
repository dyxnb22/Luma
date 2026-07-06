@preconcurrency import AppKit
import LumaCore
import LumaModules

final class ActivitySparklineView: NSView {
    nonisolated(unsafe) private var buckets: [PersistentUsageTracker.ActivityBucket] = []

    override var isFlipped: Bool { true }

    @MainActor
    func apply(buckets: [PersistentUsageTracker.ActivityBucket]) {
        self.buckets = buckets
        needsDisplay = true
    }

    nonisolated override func draw(_ dirtyRect: NSRect) {
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
    private let moduleSummaryStack = NSStackView()

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
            let rows = byModule
                .sorted { $0.value > $1.value }
                .prefix(12)
                .map { (ModuleRegistry.displayName(for: $0.key), $0.value) }
            await MainActor.run {
                sparkline7.apply(buckets: buckets7)
                sparkline30.apply(buckets: buckets30)
                renderModuleSummary(rows)
            }
        }
    }

    private func renderModuleSummary(_ rows: [(name: String, count: Int)]) {
        moduleSummaryStack.arrangedSubviews.forEach { view in
            moduleSummaryStack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        guard !rows.isEmpty else {
            let label = moduleCountLabel("No usage recorded yet.")
            moduleSummaryStack.addArrangedSubview(label)
            return
        }
        for row in rows {
            let label = moduleCountLabel("\(row.name): \(row.count)")
            moduleSummaryStack.addArrangedSubview(label)
        }
    }

    private func moduleCountLabel(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = TypographyTokens.body
        label.textColor = .labelColor
        label.lineBreakMode = .byWordWrapping
        label.preferredMaxLayoutWidth = 480
        return label
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
        stack.addArrangedSubview(sparkline7)

        stack.addArrangedSubview(section("Last 30 days"))
        sparkline30.translatesAutoresizingMaskIntoConstraints = false
        sparkline30.heightAnchor.constraint(equalToConstant: 48).isActive = true
        stack.addArrangedSubview(sparkline30)

        stack.addArrangedSubview(section("By module (30 d)"))
        moduleSummaryStack.orientation = .vertical
        moduleSummaryStack.alignment = .leading
        moduleSummaryStack.spacing = 4
        stack.addArrangedSubview(moduleSummaryStack)

        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            stack.bottomAnchor.constraint(lessThanOrEqualTo: bottomAnchor, constant: -12),
            sparkline7.widthAnchor.constraint(equalTo: stack.widthAnchor),
            sparkline30.widthAnchor.constraint(equalTo: stack.widthAnchor),
            moduleSummaryStack.widthAnchor.constraint(equalTo: stack.widthAnchor)
        ])
    }

    private func section(_ title: String) -> NSTextField {
        let label = NSTextField(labelWithString: title)
        label.font = TypographyTokens.title3
        return label
    }
}
