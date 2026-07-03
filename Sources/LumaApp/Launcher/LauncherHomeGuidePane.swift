import AppKit
import LumaCore

/// Read-only command guide for empty-query home (right column). Not a second navigable list.
@MainActor
final class LauncherHomeGuidePane: NSView {
    private let titleLabel = NSTextField(labelWithString: "")
    private let scrollView = NSScrollView()
    private let stack = NSStackView()

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        clipsToBounds = true
        isHidden = true
        GeekUIKit.configureContentSurface(self)

        titleLabel.font = TypographyTokens.callout
        titleLabel.textColor = .labelColor
        titleLabel.lineBreakMode = .byTruncatingTail
        titleLabel.maximumNumberOfLines = 1
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.translatesAutoresizingMaskIntoConstraints = false

        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.documentView = stack
        scrollView.translatesAutoresizingMaskIntoConstraints = false

        addSubview(titleLabel)
        addSubview(scrollView)

        NSLayoutConstraint.activate([
            titleLabel.topAnchor.constraint(equalTo: topAnchor, constant: 4),
            titleLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            titleLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),

            scrollView.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 8),
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: bottomAnchor),

            stack.topAnchor.constraint(equalTo: scrollView.contentView.topAnchor),
            stack.leadingAnchor.constraint(equalTo: scrollView.contentView.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: scrollView.contentView.trailingAnchor),
            stack.widthAnchor.constraint(equalTo: scrollView.contentView.widthAnchor)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func applyCatalog(_ commands: [CommandDefinition]) {
        titleLabel.stringValue = L10n.tr("home.guide.title")
        rebuild {
            appendBody(L10n.tr("home.guide.intro"))
            for command in commands {
                appendCommandBlock(command)
            }
            appendFooter()
        }
    }

    func applySelection(_ item: ResultItem) {
        titleLabel.stringValue = item.title
        rebuild {
            if let subtitle = item.subtitle, !subtitle.isEmpty {
                appendBody(subtitle)
            }
            appendMonoLine("\(L10n.tr("home.guide.return")) \(item.returnHint)")
            for action in item.secondaryActions.prefix(3) {
                appendMonoLine("\(L10n.tr("home.guide.more")) \(action.title)")
            }
            appendFooter()
        }
    }

    private func rebuild(_ build: () -> Void) {
        stack.arrangedSubviews.forEach { view in
            stack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        build()
        layoutSubtreeIfNeeded()
        GeekUIKit.syncVerticalListDocumentFrame(in: scrollView)
    }

    private func appendCommandBlock(_ command: CommandDefinition) {
        let trigger = command.primaryTrigger
        appendHeading(trigger)
        appendBody(command.resolvedDescription)
        if let line = command.helpLines.first {
            appendMonoLine(line)
        }
    }

    private func appendHeading(_ text: String) {
        let label = makeLabel(font: TypographyTokens.caption(weight: .semibold), mono: false)
        label.stringValue = text
        stack.addArrangedSubview(label)
        label.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
    }

    private func appendBody(_ text: String) {
        let label = makeLabel(font: TypographyTokens.caption, mono: false)
        label.stringValue = text
        label.maximumNumberOfLines = 0
        label.lineBreakMode = .byWordWrapping
        stack.addArrangedSubview(label)
        label.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
    }

    private func appendMonoLine(_ text: String) {
        let label = makeLabel(font: TypographyTokens.monoCaption(), mono: true)
        label.stringValue = text
        label.maximumNumberOfLines = 0
        label.lineBreakMode = .byWordWrapping
        stack.addArrangedSubview(label)
        label.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
    }

    private func appendFooter() {
        appendBody(L10n.tr("home.guide.footer"))
    }

    private func makeLabel(font: NSFont, mono: Bool) -> NSTextField {
        let label = NSTextField(labelWithString: "")
        label.font = font
        label.textColor = mono ? .secondaryLabelColor : .labelColor.withAlphaComponent(0.82)
        label.isBezeled = false
        label.isEditable = false
        label.drawsBackground = false
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }
}
