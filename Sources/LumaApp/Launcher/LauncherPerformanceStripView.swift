@preconcurrency import AppKit
import LumaCore

@MainActor
final class LauncherPerformanceStripView: NSView {
    var onPresenceChanged: ((Bool) -> Void)?

    private let titleLabel = NSTextField(labelWithString: "System")
    private let cpuPrefixLabel = NSTextField(labelWithString: "CPU")
    private let cpuValueLabel = NSTextField(labelWithString: "—")
    private let separatorLabel = NSTextField(labelWithString: "·")
    private let memoryPrefixLabel = NSTextField(labelWithString: "MEM")
    private let memoryValueLabel = NSTextField(labelWithString: "—")
    private let secondSeparatorLabel = NSTextField(labelWithString: "·")
    private let todayPrefixLabel = NSTextField(labelWithString: "Today")
    private let todayValueLabel = NSTextField(labelWithString: "—")
    private let thirdSeparatorLabel = NSTextField(labelWithString: "·")
    private let reviewPrefixLabel = NSTextField(labelWithString: "Review")
    private let reviewValueLabel = NSTextField(labelWithString: "—")
    private let metricsStack = NSStackView()
    private var heightConstraint: NSLayoutConstraint!
    private(set) var isContentVisible = true

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        GeekUIKit.installPerformanceStripSurface(on: self)

        configureLabel(titleLabel, font: TypographyTokens.caption2(weight: .semibold))
        titleLabel.textColor = ColorTokens.performanceStripNormalText

        for label in [cpuPrefixLabel, memoryPrefixLabel, todayPrefixLabel, reviewPrefixLabel] {
            configureLabel(label, font: TypographyTokens.caption2(weight: .medium))
            label.textColor = ColorTokens.performanceStripNormalText
        }

        for label in [cpuValueLabel, memoryValueLabel, todayValueLabel, reviewValueLabel] {
            configureLabel(label, font: TypographyTokens.monoCaption())
            label.textColor = ColorTokens.performanceStripMetricText
        }

        for label in [separatorLabel, secondSeparatorLabel, thirdSeparatorLabel] {
            configureLabel(label, font: TypographyTokens.caption2())
            label.textColor = ColorTokens.performanceStripNormalText
        }

        metricsStack.orientation = .horizontal
        metricsStack.alignment = .centerY
        metricsStack.spacing = LauncherChromeTokens.performanceMetricGap
        metricsStack.translatesAutoresizingMaskIntoConstraints = false
        metricsStack.addArrangedSubview(metricGroup(prefix: cpuPrefixLabel, value: cpuValueLabel))
        metricsStack.addArrangedSubview(separatorLabel)
        metricsStack.addArrangedSubview(metricGroup(prefix: memoryPrefixLabel, value: memoryValueLabel))
        metricsStack.addArrangedSubview(secondSeparatorLabel)
        metricsStack.addArrangedSubview(metricGroup(prefix: todayPrefixLabel, value: todayValueLabel))
        metricsStack.addArrangedSubview(thirdSeparatorLabel)
        metricsStack.addArrangedSubview(metricGroup(prefix: reviewPrefixLabel, value: reviewValueLabel))

        titleLabel.translatesAutoresizingMaskIntoConstraints = false
        addSubview(titleLabel)
        addSubview(metricsStack)

        heightConstraint = heightAnchor.constraint(equalToConstant: LauncherChromeTokens.performanceStripHeight)
        heightConstraint.isActive = true

        NSLayoutConstraint.activate([
            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 6),
            titleLabel.centerYAnchor.constraint(equalTo: centerYAnchor),

            metricsStack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -6),
            metricsStack.centerYAnchor.constraint(equalTo: centerYAnchor),
            metricsStack.leadingAnchor.constraint(greaterThanOrEqualTo: titleLabel.trailingAnchor, constant: 12)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func apply(_ presentation: PerformanceStripPresentation) {
        cpuValueLabel.stringValue = presentation.cpuText
        memoryValueLabel.stringValue = presentation.memoryText
        todayValueLabel.stringValue = presentation.todayText
        reviewValueLabel.stringValue = presentation.reviewText

        cpuValueLabel.textColor = color(for: presentation.cpuEmphasis)
        memoryValueLabel.textColor = color(for: presentation.memoryEmphasis)
        todayValueLabel.textColor = color(for: presentation.todayEmphasis)
        reviewValueLabel.textColor = color(for: presentation.reviewEmphasis)
    }

    func setContentVisible(_ visible: Bool) {
        guard isContentVisible != visible else { return }
        isContentVisible = visible
        heightConstraint.constant = visible ? LauncherChromeTokens.performanceStripHeight : 0
        alphaValue = visible ? 1 : 0
        isHidden = !visible
        onPresenceChanged?(visible)
    }

    private func metricGroup(prefix: NSTextField, value: NSTextField) -> NSStackView {
        let group = NSStackView(views: [prefix, value])
        group.orientation = .horizontal
        group.alignment = .centerY
        group.spacing = 4
        return group
    }

    private func configureLabel(_ label: NSTextField, font: NSFont) {
        label.font = font
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.setContentHuggingPriority(.required, for: .horizontal)
        label.setContentCompressionResistancePriority(.required, for: .horizontal)
    }

    private func color(for emphasis: PerformanceStripEmphasis) -> NSColor {
        switch emphasis {
        case .normal:
            return ColorTokens.performanceStripMetricText
        case .elevated:
            return ColorTokens.performanceStripElevatedMetricText
        case .warning:
            return ColorTokens.performanceStripWarningAccent
        }
    }
}
