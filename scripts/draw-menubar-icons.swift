// Draws assets/menubar/keepy-uppy-{on,off}.png. Usage: swift scripts/draw-menubar-icons.swift assets/menubar
import AppKit

// 36 px canvas = 18 pt @2x. y grows upward in AppKit.
let size = 36

func pentagon(center c: CGPoint, radius r: CGFloat) -> NSBezierPath {
    let p = NSBezierPath()
    for i in 0..<5 {
        let a = CGFloat.pi / 2 + CGFloat(i) * 2 * .pi / 5
        let pt = CGPoint(x: c.x + r * cos(a), y: c.y + r * sin(a))
        i == 0 ? p.move(to: pt) : p.line(to: pt)
    }
    p.close()
    return p
}

func ball(center c: CGPoint, radius r: CGFloat, filled: Bool) {
    let disc = NSBezierPath(ovalIn: CGRect(x: c.x - r, y: c.y - r, width: 2 * r, height: 2 * r))
    let pent = pentagon(center: c, radius: r * 0.38)
    if filled {
        NSColor.black.setFill(); disc.fill()
        // Cut the pattern out of the solid ball.
        NSGraphicsContext.current!.compositingOperation = .clear
        pent.fill()
        let seams = NSBezierPath(); seams.lineWidth = 1.6
        for i in 0..<5 {
            let a = CGFloat.pi / 2 + CGFloat(i) * 2 * .pi / 5
            seams.move(to: CGPoint(x: c.x + r * 0.38 * cos(a), y: c.y + r * 0.38 * sin(a)))
            seams.line(to: CGPoint(x: c.x + r * 1.1 * cos(a), y: c.y + r * 1.1 * sin(a)))
        }
        seams.stroke()
        NSGraphicsContext.current!.compositingOperation = .sourceOver
    } else {
        let inset: CGFloat = 1.25
        let ring = NSBezierPath(ovalIn: CGRect(x: c.x - r + inset, y: c.y - r + inset, width: 2 * (r - inset), height: 2 * (r - inset)))
        ring.lineWidth = 2.5
        NSColor.black.setStroke(); ring.stroke()
        NSColor.black.setFill(); pent.fill()
    }
}

func ground() {
    NSColor.black.setFill()
    NSBezierPath(roundedRect: CGRect(x: 3, y: 1, width: 30, height: 3), xRadius: 1.5, yRadius: 1.5).fill()
}

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

// On: solid ball high up, motion marks underneath, clear gap to the ground.
render("keepy-uppy-on") {
    ball(center: CGPoint(x: 18, y: 24), radius: 10, filled: true)
    NSColor.black.setStroke()
    for (x0, x1) in [(8.0, 11.0), (28.0, 25.0)] {
        let m = NSBezierPath(); m.lineWidth = 2; m.lineCapStyle = .round
        m.move(to: CGPoint(x: x0, y: 9)); m.line(to: CGPoint(x: x1, y: 12)); m.stroke()
    }
    let mid = NSBezierPath(); mid.lineWidth = 2; mid.lineCapStyle = .round
    mid.move(to: CGPoint(x: 18, y: 8)); mid.line(to: CGPoint(x: 18, y: 11)); mid.stroke()
    ground()
}

// Off: hollow ball resting on the ground.
render("keepy-uppy-off") {
    ball(center: CGPoint(x: 18, y: 14), radius: 10, filled: false)
    ground()
}
