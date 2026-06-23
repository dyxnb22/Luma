import AppKit

/// Template menu-bar mark for Luma — terminal brackets + light-node "L".
enum LumaMenuBarIcon {
    enum State: Equatable {
        case normal
        case vaultLocked
        case vaultUnlocked
        case hotkeyWarning
    }

    static func make(state: State, pointSize: CGFloat = 18) -> NSImage {
        let size = NSSize(width: pointSize, height: pointSize)
        let image = NSImage(size: size, flipped: false) { rect in
            draw(in: rect, state: state)
            return true
        }
        image.isTemplate = true
        return image
    }

    private static func draw(in rect: NSRect, state: State) {
        let s = min(rect.width, rect.height)
        let ox = rect.midX - s * 0.5
        let oy = rect.midY - s * 0.5
        let inset = s * 0.14
        let stroke = max(1.15, s * 0.085)

        let frame = NSRect(x: ox + inset, y: oy + inset, width: s - inset * 2, height: s - inset * 2)
        let corner = s * 0.18

        NSColor.black.setStroke()
        NSColor.black.setFill()

        // Terminal corner brackets.
        let brackets = NSBezierPath()
        brackets.lineWidth = stroke
        brackets.lineCapStyle = .square

        brackets.move(to: NSPoint(x: frame.minX, y: frame.minY + corner))
        brackets.line(to: NSPoint(x: frame.minX, y: frame.minY))
        brackets.line(to: NSPoint(x: frame.minX + corner, y: frame.minY))

        brackets.move(to: NSPoint(x: frame.maxX - corner, y: frame.maxY))
        brackets.line(to: NSPoint(x: frame.maxX, y: frame.maxY))
        brackets.line(to: NSPoint(x: frame.maxX, y: frame.maxY - corner))
        brackets.stroke()

        // Monogram L + light node.
        let stemX = frame.minX + frame.width * 0.30
        let topY = frame.maxY - frame.height * 0.18
        let baseY = frame.minY + frame.height * 0.16
        let barEndX = frame.minX + frame.width * 0.72

        let lPath = NSBezierPath()
        lPath.lineWidth = stroke
        lPath.lineCapStyle = .round
        lPath.lineJoinStyle = .round
        lPath.move(to: NSPoint(x: stemX, y: topY))
        lPath.line(to: NSPoint(x: stemX, y: baseY))
        lPath.line(to: NSPoint(x: barEndX, y: baseY))
        lPath.stroke()

        let nodeRadius = s * 0.085
        let nodeCenter = NSPoint(x: stemX, y: topY + nodeRadius * 0.35)
        let node = NSBezierPath(ovalIn: NSRect(
            x: nodeCenter.x - nodeRadius,
            y: nodeCenter.y - nodeRadius,
            width: nodeRadius * 2,
            height: nodeRadius * 2
        ))
        node.fill()

        // Light rays — geek "signal" accent.
        let rays = NSBezierPath()
        rays.lineWidth = max(0.9, stroke * 0.72)
        rays.lineCapStyle = .round
        let rayStart = NSPoint(x: nodeCenter.x + nodeRadius * 0.9, y: nodeCenter.y + nodeRadius * 0.2)
        rays.move(to: rayStart)
        rays.line(to: NSPoint(x: rayStart.x + s * 0.16, y: rayStart.y + s * 0.10))
        rays.move(to: rayStart)
        rays.line(to: NSPoint(x: rayStart.x + s * 0.14, y: rayStart.y - s * 0.02))
        rays.stroke()

        switch state {
        case .normal:
            break
        case .vaultUnlocked:
            let pulse = NSBezierPath(ovalIn: NSRect(
                x: nodeCenter.x - nodeRadius * 1.45,
                y: nodeCenter.y - nodeRadius * 1.45,
                width: nodeRadius * 2.9,
                height: nodeRadius * 2.9
            ))
            pulse.lineWidth = max(0.8, stroke * 0.65)
            pulse.stroke()
        case .vaultLocked:
            drawLockBadge(in: frame, scale: s, stroke: stroke)
        case .hotkeyWarning:
            drawWarningBadge(in: frame, scale: s, stroke: stroke)
        }
    }

    private static func drawLockBadge(in frame: NSRect, scale s: CGFloat, stroke: CGFloat) {
        let w = s * 0.22
        let h = s * 0.18
        let x = frame.maxX - w - s * 0.02
        let y = frame.minY + s * 0.02
        let body = NSRect(x: x, y: y, width: w, height: h * 0.62)
        let shackleW = w * 0.62
        let shackleH = h * 0.55
        let shackleX = x + (w - shackleW) * 0.5
        let shackleY = body.maxY - shackleH * 0.35

        let shackle = NSBezierPath()
        shackle.lineWidth = max(0.9, stroke * 0.7)
        shackle.lineCapStyle = .round
        shackle.appendArc(
            withCenter: NSPoint(x: shackleX + shackleW * 0.5, y: shackleY),
            radius: shackleW * 0.5,
            startAngle: 0,
            endAngle: 180
        )
        shackle.stroke()

        let lockBody = NSBezierPath(roundedRect: body, xRadius: 1.2, yRadius: 1.2)
        lockBody.fill()
    }

    private static func drawWarningBadge(in frame: NSRect, scale s: CGFloat, stroke: CGFloat) {
        let side = s * 0.24
        let x = frame.maxX - side - s * 0.01
        let y = frame.minY + s * 0.01
        let tri = NSBezierPath()
        tri.move(to: NSPoint(x: x + side * 0.5, y: y + side))
        tri.line(to: NSPoint(x: x, y: y))
        tri.line(to: NSPoint(x: x + side, y: y))
        tri.close()
        tri.fill()

        let mark = NSBezierPath()
        mark.lineWidth = max(0.9, stroke * 0.55)
        mark.lineCapStyle = .round
        let cx = x + side * 0.5
        mark.move(to: NSPoint(x: cx, y: y + side * 0.22))
        mark.line(to: NSPoint(x: cx, y: y + side * 0.52))
        mark.move(to: NSPoint(x: cx, y: y + side * 0.66))
        mark.line(to: NSPoint(x: cx, y: y + side * 0.66))
        NSColor.white.setStroke()
        mark.stroke()
    }
}
