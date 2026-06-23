import AppKit

@MainActor
final class FlippedStackView: NSStackView {
    override var isFlipped: Bool { true }
}

@MainActor
final class FeatureFlowView: NSView {
    var cardViews: [WidgetFeatureCard] = [] {
        didSet {
            resizeDocument()
            needsLayout = true
        }
    }

    override var isFlipped: Bool { true }

    override func layout() {
        super.layout()
        resizeDocument()
        layoutCards()
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        resizeDocument()
    }

    private func layoutCards() {
        guard !cardViews.isEmpty else { return }
        let layout = layoutMetrics(for: bounds.width)
        for (index, card) in cardViews.enumerated() {
            let column = index % layout.columns
            let row = index / layout.columns
            let x = CGFloat(column) * (layout.cardWidth + layout.gap)
            let y = CGFloat(row) * (layout.cardHeight + layout.gap)
            card.frame = CGRect(x: x, y: y, width: layout.cardWidth, height: layout.cardHeight)
        }
    }

    func contentHeight(for width: CGFloat) -> CGFloat {
        guard !cardViews.isEmpty else { return 0 }
        let layout = layoutMetrics(for: width)
        let rows = Int(ceil(Double(cardViews.count) / Double(layout.columns)))
        return CGFloat(rows) * layout.cardHeight + CGFloat(max(0, rows - 1)) * layout.gap
    }

    private func resizeDocument() {
        guard let scrollView = enclosingScrollView else { return }
        let width = scrollView.contentView.bounds.width
        let height = max(scrollView.contentView.bounds.height, contentHeight(for: width))
        if abs(frame.width - width) > 0.5 || abs(frame.height - height) > 0.5 {
            frame = CGRect(origin: .zero, size: NSSize(width: width, height: height))
        }
    }

    private func layoutMetrics(for width: CGFloat) -> (columns: Int, cardWidth: CGFloat, cardHeight: CGFloat, gap: CGFloat) {
        let gap: CGFloat = 16
        let minimumWidth: CGFloat = 190
        let maximumWidth: CGFloat = 248
        let cardHeight: CGFloat = 132
        let availableWidth = max(width, minimumWidth)
        let columns = max(1, Int((availableWidth + gap) / (minimumWidth + gap)))
        let rawWidth = floor((availableWidth - CGFloat(columns - 1) * gap) / CGFloat(columns))
        let cardWidth = min(maximumWidth, max(minimumWidth, rawWidth))
        return (columns, cardWidth, cardHeight, gap)
    }
}
