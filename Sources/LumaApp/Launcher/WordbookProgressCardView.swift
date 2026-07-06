@preconcurrency import AppKit
import LumaModules

@MainActor
final class WordbookProgressCardView: NSView {
    private let titleLabel = NSTextField(labelWithString: "Today's Plan")
    private let progressBar = NSProgressIndicator()
    private let statsLabel = NSTextField(wrappingLabelWithString: "")
    private let masteredLabel = NSTextField(labelWithString: "")

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.cornerRadius = 12
        layer?.cornerCurve = .continuous
        layer?.backgroundColor = NSColor.controlBackgroundColor.withAlphaComponent(0.92).cgColor
        layer?.borderWidth = 1
        layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.35).cgColor
        setup()
        showLoading()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func setup() {
        titleLabel.font = .systemFont(ofSize: 13, weight: .semibold)
        titleLabel.translatesAutoresizingMaskIntoConstraints = false
        progressBar.isIndeterminate = false
        progressBar.maxValue = 100
        progressBar.translatesAutoresizingMaskIntoConstraints = false
        statsLabel.font = .systemFont(ofSize: 12)
        statsLabel.textColor = .labelColor
        statsLabel.translatesAutoresizingMaskIntoConstraints = false
        masteredLabel.font = .systemFont(ofSize: 11)
        masteredLabel.textColor = .secondaryLabelColor
        masteredLabel.translatesAutoresizingMaskIntoConstraints = false
        addSubview(titleLabel)
        addSubview(progressBar)
        addSubview(statsLabel)
        addSubview(masteredLabel)
        NSLayoutConstraint.activate([
            titleLabel.topAnchor.constraint(equalTo: topAnchor, constant: 14),
            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            titleLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16),
            progressBar.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 10),
            progressBar.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            progressBar.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),
            statsLabel.topAnchor.constraint(equalTo: progressBar.bottomAnchor, constant: 8),
            statsLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            statsLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16),
            masteredLabel.topAnchor.constraint(equalTo: statsLabel.bottomAnchor, constant: 4),
            masteredLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            masteredLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16),
            masteredLabel.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -14)
        ])
    }

    func showLoading() {
        statsLabel.stringValue = "Loading…"
        masteredLabel.stringValue = ""
        progressBar.doubleValue = 0
    }

    func apply(_ snapshot: WordbookProgressSnapshot) {
        let ratio = snapshot.total > 0 ? Double(snapshot.mastered) / Double(snapshot.total) : 0
        progressBar.doubleValue = ratio * 100
        let accuracy = Int(snapshot.accuracyToday * 100)
        statsLabel.stringValue = "\(snapshot.dueToday) due · \(snapshot.dailyNewSeen)/\(snapshot.dailyNewLimit) new · \(accuracy)% accuracy · \(snapshot.todayMastered) mastered today"
        var masteredLine = "Mastered \(snapshot.mastered)/\(snapshot.total)"
        if snapshot.streakDays >= 1 {
            masteredLine += " · Streak \(snapshot.streakDays)d"
        }
        masteredLabel.stringValue = masteredLine
    }
}
