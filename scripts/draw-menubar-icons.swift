// Draws the Watchful Lid menu bar icons. Usage: swift scripts/draw-menubar-icons.swift assets/menubar
import AppKit

// 36 px canvas = 18 pt @2x. y grows upward in AppKit.
let size = 36

func render(_ name: String, _ draw: () -> Void) {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw()
    NSGraphicsContext.restoreGraphicsState()
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1] + "/" + name + ".png"))
}

// Watchful Lid: a laptop lid (trapezoid) over a base bar.
func lidPath() -> NSBezierPath {
    let p = NSBezierPath()
    p.move(to: CGPoint(x: 4, y: 9)); p.line(to: CGPoint(x: 32, y: 9))
    p.line(to: CGPoint(x: 28, y: 32)); p.line(to: CGPoint(x: 8, y: 32)); p.close()
    p.lineJoinStyle = .round
    return p
}

func base() {
    NSColor.black.setFill()
    NSBezierPath(roundedRect: CGRect(x: 1, y: 3, width: 34, height: 3.5), xRadius: 1.75, yRadius: 1.75).fill()
}

func quad(_ a: CGPoint, _ c: CGPoint, _ b: CGPoint, _ t: CGFloat) -> CGPoint {
    let u = 1 - t
    return CGPoint(x: u * u * a.x + 2 * u * t * c.x + t * t * b.x, y: u * u * a.y + 2 * u * t * c.y + t * t * b.y)
}

func cubic(_ a: CGPoint, _ c1: CGPoint, _ c2: CGPoint, _ b: CGPoint, _ t: CGFloat) -> CGPoint {
    let u = 1 - t
    let x = u * u * u * a.x + 3 * u * u * t * c1.x + 3 * u * t * t * c2.x + t * t * t * b.x
    let y = u * u * u * a.y + 3 * u * u * t * c1.y + 3 * u * t * t * c2.y + t * t * t * b.y
    return CGPoint(x: x, y: y)
}

// On: hollow lid (same as off) with an open eye, solid pupil and upward lashes.
render("watchful-lid-on") {
    base()
    let lid = lidPath()
    lid.lineWidth = 2.6; NSColor.black.setStroke(); lid.stroke()
    let l = CGPoint(x: 8.5, y: 19), r = CGPoint(x: 27.5, y: 19)
    let up1 = CGPoint(x: 13, y: 25.5), up2 = CGPoint(x: 23, y: 25.5)
    let eye = NSBezierPath(); eye.lineWidth = 2.4; eye.lineJoinStyle = .round
    eye.move(to: l)
    eye.curve(to: r, controlPoint1: up1, controlPoint2: up2)
    eye.curve(to: l, controlPoint1: CGPoint(x: 23, y: 12.5), controlPoint2: CGPoint(x: 13, y: 12.5))
    eye.stroke()
    NSColor.black.setFill()
    NSBezierPath(ovalIn: CGRect(x: 18 - 4.2, y: 19 - 4.2, width: 8.4, height: 8.4)).fill()
    for (t, dx) in [(0.25, -1.6), (0.5, 0.0), (0.75, 1.6)] {
        let p = cubic(l, up1, up2, r, t)
        let lash = NSBezierPath(); lash.lineWidth = 2.2; lash.lineCapStyle = .round
        lash.move(to: p); lash.line(to: CGPoint(x: p.x + dx, y: p.y + 3.2)); lash.stroke()
    }
}

// Off: hollow lid with a thick closed eyelid and downward lashes.
render("watchful-lid-off") {
    base()
    let lid = lidPath()
    lid.lineWidth = 2.6; NSColor.black.setStroke(); lid.stroke()
    let a = CGPoint(x: 9, y: 22), c = CGPoint(x: 18, y: 14), b = CGPoint(x: 27, y: 22)
    let lidLine = NSBezierPath(); lidLine.lineWidth = 2.8; lidLine.lineCapStyle = .round
    lidLine.move(to: a); lidLine.curve(to: b, controlPoint1: CGPoint(x: 14, y: 16.7), controlPoint2: CGPoint(x: 22, y: 16.7))
    lidLine.stroke()
    for (t, dx) in [(0.2, -2.2), (0.5, 0.0), (0.8, 2.2)] {
        let p = quad(a, c, b, t)
        let lash = NSBezierPath(); lash.lineWidth = 2.2; lash.lineCapStyle = .round
        lash.move(to: p); lash.line(to: CGPoint(x: p.x + dx, y: p.y - 4.2)); lash.stroke()
    }
}
