import AppKit

/// Standard detail-view chrome: 16 pt margins, optional toolbar, scrollable content, optional footer.
@MainActor
final class BaseDetailContainer: NSView {
    static let margin: CGFloat = 16

    private let toolbarHost = NSView()
    private let bodyHost = NSView()
    private let scrollView = NSScrollView()
    private let footerHost = NSView()
    private var toolbarHeight: NSLayoutConstraint?
    private var footerHeight: NSLayoutConstraint?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        installChrome()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func setToolbar(_ view: NSView, height: CGFloat = 36) {
        toolbarHost.subviews.forEach { $0.removeFromSuperview() }
        view.translatesAutoresizingMaskIntoConstraints = false
        toolbarHost.addSubview(view)
        NSLayoutConstraint.activate([
            view.topAnchor.constraint(equalTo: toolbarHost.topAnchor),
            view.leadingAnchor.constraint(equalTo: toolbarHost.leadingAnchor),
            view.trailingAnchor.constraint(equalTo: toolbarHost.trailingAnchor),
            view.bottomAnchor.constraint(equalTo: toolbarHost.bottomAnchor)
        ])
        toolbarHeight?.constant = height
        toolbarHost.isHidden = false
    }

    func setContent(_ view: NSView, embedInScroll: Bool = true) {
        bodyHost.subviews.forEach { $0.removeFromSuperview() }
        scrollView.documentView = nil

        if embedInScroll {
            scrollView.translatesAutoresizingMaskIntoConstraints = false
            bodyHost.addSubview(scrollView)
            NSLayoutConstraint.activate([
                scrollView.topAnchor.constraint(equalTo: bodyHost.topAnchor),
                scrollView.leadingAnchor.constraint(equalTo: bodyHost.leadingAnchor),
                scrollView.trailingAnchor.constraint(equalTo: bodyHost.trailingAnchor),
                scrollView.bottomAnchor.constraint(equalTo: bodyHost.bottomAnchor)
            ])
            view.translatesAutoresizingMaskIntoConstraints = false
            scrollView.documentView = view
            NSLayoutConstraint.activate([
                view.leadingAnchor.constraint(equalTo: scrollView.contentView.leadingAnchor),
                view.trailingAnchor.constraint(equalTo: scrollView.contentView.trailingAnchor),
                view.topAnchor.constraint(equalTo: scrollView.contentView.topAnchor),
                view.widthAnchor.constraint(equalTo: scrollView.widthAnchor, constant: -Self.margin * 2)
            ])
        } else {
            view.translatesAutoresizingMaskIntoConstraints = false
            bodyHost.addSubview(view)
            NSLayoutConstraint.activate([
                view.topAnchor.constraint(equalTo: bodyHost.topAnchor),
                view.leadingAnchor.constraint(equalTo: bodyHost.leadingAnchor),
                view.trailingAnchor.constraint(equalTo: bodyHost.trailingAnchor),
                view.bottomAnchor.constraint(equalTo: bodyHost.bottomAnchor)
            ])
        }
    }

    func setFooter(_ view: NSView?, height: CGFloat = 28) {
        footerHost.subviews.forEach { $0.removeFromSuperview() }
        guard let view else {
            footerHost.isHidden = true
            footerHeight?.constant = 0
            return
        }
        view.translatesAutoresizingMaskIntoConstraints = false
        footerHost.addSubview(view)
        NSLayoutConstraint.activate([
            view.topAnchor.constraint(equalTo: footerHost.topAnchor),
            view.leadingAnchor.constraint(equalTo: footerHost.leadingAnchor),
            view.trailingAnchor.constraint(equalTo: footerHost.trailingAnchor),
            view.bottomAnchor.constraint(equalTo: footerHost.bottomAnchor)
        ])
        footerHeight?.constant = height
        footerHost.isHidden = false
    }

    private func installChrome() {
        toolbarHost.translatesAutoresizingMaskIntoConstraints = false
        scrollView.hasVerticalScroller = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        bodyHost.translatesAutoresizingMaskIntoConstraints = false
        footerHost.translatesAutoresizingMaskIntoConstraints = false
        footerHost.isHidden = true

        addSubview(toolbarHost)
        addSubview(bodyHost)
        addSubview(footerHost)

        toolbarHeight = toolbarHost.heightAnchor.constraint(equalToConstant: 36)
        footerHeight = footerHost.heightAnchor.constraint(equalToConstant: 0)

        NSLayoutConstraint.activate([
            toolbarHost.topAnchor.constraint(equalTo: topAnchor, constant: Self.margin),
            toolbarHost.leadingAnchor.constraint(equalTo: leadingAnchor, constant: Self.margin),
            toolbarHost.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -Self.margin),
            toolbarHeight!,

            bodyHost.topAnchor.constraint(equalTo: toolbarHost.bottomAnchor, constant: 8),
            bodyHost.leadingAnchor.constraint(equalTo: leadingAnchor, constant: Self.margin),
            bodyHost.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -Self.margin),
            bodyHost.bottomAnchor.constraint(equalTo: footerHost.topAnchor, constant: -8),

            footerHost.leadingAnchor.constraint(equalTo: leadingAnchor, constant: Self.margin),
            footerHost.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -Self.margin),
            footerHost.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -Self.margin),
            footerHeight!
        ])
    }
}
