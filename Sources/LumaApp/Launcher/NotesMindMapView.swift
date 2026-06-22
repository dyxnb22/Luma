import AppKit
import LumaModules

@MainActor
final class NotesMindMapView: NSView {
    private struct PositionedNode {
        let node: NotesNode
        let rect: CGRect
        let parentPath: String?
    }

    private var root: NotesNode?
    private var expandedPaths = Set<String>()
    private var positionedNodes: [PositionedNode] = []

    private let nodeSize = CGSize(width: 168, height: 48)
    private let horizontalSpacing: CGFloat = 92
    private let verticalSpacing: CGFloat = 20
    private let padding: CGFloat = 28

    override var isFlipped: Bool { true }

    func reload(root: NotesNode?) {
        self.root = root
        expandedPaths = root.map { collectFolderPaths(from: $0) } ?? []
        rebuildLayout()
    }

    func expandAll() {
        expandedPaths = root.map { collectFolderPaths(from: $0) } ?? []
        rebuildLayout()
    }

    func collapseAll() {
        expandedPaths = root.map { [$0.path] } ?? []
        rebuildLayout()
    }

    override func draw(_ dirtyRect: NSRect) {
        NSColor.textBackgroundColor.setFill()
        bounds.fill()

        guard !positionedNodes.isEmpty else { return }
        drawConnections()
        for item in positionedNodes {
            drawNode(item)
        }
    }

    private func rebuildLayout() {
        positionedNodes.removeAll()
        guard let root else {
            frame.size = CGSize(width: 640, height: 360)
            needsDisplay = true
            return
        }

        var maxDepth = 0
        let totalHeight = layout(node: root, depth: 0, y: padding, parentPath: nil, maxDepth: &maxDepth)
        let width = padding * 2 + CGFloat(maxDepth + 1) * nodeSize.width + CGFloat(maxDepth) * horizontalSpacing
        frame.size = CGSize(width: max(width, 720), height: max(totalHeight + padding, 420))
        needsDisplay = true
    }

    @discardableResult
    private func layout(
        node: NotesNode,
        depth: Int,
        y: CGFloat,
        parentPath: String?,
        maxDepth: inout Int
    ) -> CGFloat {
        maxDepth = max(maxDepth, depth)
        let children = visibleChildren(of: node)

        if children.isEmpty {
            let rect = CGRect(
                x: padding + CGFloat(depth) * (nodeSize.width + horizontalSpacing),
                y: y,
                width: nodeSize.width,
                height: nodeSize.height
            )
            positionedNodes.append(PositionedNode(node: node, rect: rect, parentPath: parentPath))
            return nodeSize.height + verticalSpacing
        }

        var childY = y
        let firstChildStart = childY
        for child in children {
            childY += layout(node: child, depth: depth + 1, y: childY, parentPath: node.path, maxDepth: &maxDepth)
        }
        let childBlockHeight = childY - firstChildStart - verticalSpacing
        let nodeY = firstChildStart + max(0, (childBlockHeight - nodeSize.height) / 2)
        let rect = CGRect(
            x: padding + CGFloat(depth) * (nodeSize.width + horizontalSpacing),
            y: nodeY,
            width: nodeSize.width,
            height: nodeSize.height
        )
        positionedNodes.append(PositionedNode(node: node, rect: rect, parentPath: parentPath))
        return max(nodeSize.height + verticalSpacing, childY - y)
    }

    private func visibleChildren(of node: NotesNode) -> [NotesNode] {
        guard node.kind == .folder, expandedPaths.contains(node.path) else { return [] }
        return node.children
    }

    private func collectFolderPaths(from node: NotesNode) -> Set<String> {
        var paths = Set<String>()
        if node.kind == .folder {
            paths.insert(node.path)
        }
        for child in node.children {
            paths.formUnion(collectFolderPaths(from: child))
        }
        return paths
    }

    private func drawConnections() {
        let nodesByPath = Dictionary(uniqueKeysWithValues: positionedNodes.map { ($0.node.path, $0) })
        NSColor.controlAccentColor.withAlphaComponent(0.34).setStroke()

        for item in positionedNodes {
            guard let parentPath = item.parentPath, let parent = nodesByPath[parentPath] else { continue }
            let start = CGPoint(x: parent.rect.maxX, y: parent.rect.midY)
            let end = CGPoint(x: item.rect.minX, y: item.rect.midY)
            let midX = start.x + (end.x - start.x) * 0.5

            let path = NSBezierPath()
            path.lineWidth = 2
            path.move(to: start)
            path.curve(to: end, controlPoint1: CGPoint(x: midX, y: start.y), controlPoint2: CGPoint(x: midX, y: end.y))
            path.stroke()
        }
    }

    private func drawNode(_ item: PositionedNode) {
        let fill: NSColor = item.node.kind == .folder
            ? NSColor.controlAccentColor.withAlphaComponent(0.12)
            : NSColor.windowBackgroundColor
        let stroke: NSColor = item.node.kind == .folder
            ? NSColor.controlAccentColor.withAlphaComponent(0.48)
            : NSColor.separatorColor

        let path = NSBezierPath(roundedRect: item.rect, xRadius: 10, yRadius: 10)
        fill.setFill()
        path.fill()
        stroke.setStroke()
        path.lineWidth = 1
        path.stroke()

        let symbol = item.node.kind == .folder ? "folder" : "doc.text"
        let image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        let iconRect = CGRect(x: item.rect.minX + 12, y: item.rect.midY - 8, width: 16, height: 16)
        image?.draw(in: iconRect, from: .zero, operation: .sourceOver, fraction: 0.75)

        let title = item.node.name as NSString
        let attributes: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 13, weight: item.node.kind == .folder ? .semibold : .medium),
            .foregroundColor: NSColor.labelColor
        ]
        let textRect = CGRect(x: item.rect.minX + 36, y: item.rect.minY + 15, width: item.rect.width - 48, height: 18)
        title.draw(in: textRect, withAttributes: attributes)
    }
}
