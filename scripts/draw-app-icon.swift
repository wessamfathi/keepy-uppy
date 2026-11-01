// Draws the Watchful Lid app icon as a 1024 px PNG.
// Usage: swift scripts/draw-app-icon.swift out.png   (scripts/build-app-icon.sh makes the .icns)
import AppKit

let S: CGFloat = 1024
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(S), pixelsHigh: Int(S), bitsPerSample: 8,
                           samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                           colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let ctx = NSGraphicsContext.current!.cgContext

func rgb(_ hex: UInt32, _ a: CGFloat = 1) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255, alpha: a)
}

// Background tile on Apple's 824 px icon grid, with a soft drop shadow.
let tile = NSBezierPath(roundedRect: CGRect(x: 100, y: 100, width: 824, height: 824), xRadius: 185, yRadius: 185)
NSGraphicsContext.saveGraphicsState()
let shadow = NSShadow()
shadow.shadowColor = rgb(0x000000, 0.35); shadow.shadowBlurRadius = 28; shadow.shadowOffset = NSSize(width: 0, height: -12)
shadow.set()
rgb(0x10183F).setFill(); tile.fill()
NSGraphicsContext.restoreGraphicsState()
NSGraphicsContext.saveGraphicsState()
tile.addClip()
NSGradient(colors: [rgb(0x3346B8), rgb(0x1A2366), rgb(0x0B1030)])!.draw(in: tile, angle: -90)
// Glow behind the lid.
NSGradient(colors: [rgb(0x4FD8FF, 0.45), rgb(0x4FD8FF, 0)])!
    .draw(fromCenter: CGPoint(x: 512, y: 540), radius: 0, toCenter: CGPoint(x: 512, y: 540), radius: 420, options: [])
NSGraphicsContext.restoreGraphicsState()

// Laptop base.
let base = NSBezierPath(roundedRect: CGRect(x: 222, y: 238, width: 580, height: 52), xRadius: 26, yRadius: 26)
NSGradient(colors: [rgb(0xD9DEE7), rgb(0x8E96A6)])!.draw(in: base, angle: -90)

// Lid: trapezoid, brushed silver.
let lid = NSBezierPath()
lid.move(to: CGPoint(x: 262, y: 310)); lid.line(to: CGPoint(x: 762, y: 310))
lid.line(to: CGPoint(x: 704, y: 760)); lid.line(to: CGPoint(x: 320, y: 760)); lid.close()
NSGraphicsContext.saveGraphicsState()
let lidShadow = NSShadow()
lidShadow.shadowColor = rgb(0x000000, 0.4); lidShadow.shadowBlurRadius = 24; lidShadow.shadowOffset = NSSize(width: 0, height: -10)
lidShadow.set()
rgb(0xC9CFDA).setFill(); lid.fill()
NSGraphicsContext.restoreGraphicsState()
NSGradient(colors: [rgb(0xF2F4F8), rgb(0xC3C9D4), rgb(0x9AA2B2)])!.draw(in: lid, angle: -90)

// Eye: almond with a soft cyan glow.
let c = CGPoint(x: 512, y: 532)
let eye = NSBezierPath()
eye.move(to: CGPoint(x: c.x - 190, y: c.y))
eye.curve(to: CGPoint(x: c.x + 190, y: c.y), controlPoint1: CGPoint(x: c.x - 100, y: c.y + 150), controlPoint2: CGPoint(x: c.x + 100, y: c.y + 150))
eye.curve(to: CGPoint(x: c.x - 190, y: c.y), controlPoint1: CGPoint(x: c.x + 100, y: c.y - 150), controlPoint2: CGPoint(x: c.x - 100, y: c.y - 150))
NSGraphicsContext.saveGraphicsState()
let glow = NSShadow()
glow.shadowColor = rgb(0x4FD8FF, 0.9); glow.shadowBlurRadius = 40; glow.shadowOffset = .zero
glow.set()
rgb(0x0E1433).setFill(); eye.fill()
NSGraphicsContext.restoreGraphicsState()
NSGraphicsContext.saveGraphicsState()
eye.addClip()
NSGradient(colors: [rgb(0xFFFFFF), rgb(0xE4ECF7)])!.draw(in: eye, angle: -90)
// Iris and pupil.
let irisR: CGFloat = 92
let iris = NSBezierPath(ovalIn: CGRect(x: c.x - irisR, y: c.y - irisR, width: 2 * irisR, height: 2 * irisR))
NSGradient(colors: [rgb(0x7CEBFF), rgb(0x1FA2FF), rgb(0x1640C8)])!
    .draw(in: iris, relativeCenterPosition: NSPoint(x: 0, y: 0.15))
rgb(0x0A1030).setFill()
NSBezierPath(ovalIn: CGRect(x: c.x - 42, y: c.y - 42, width: 84, height: 84)).fill()
rgb(0xFFFFFF, 0.95).setFill()
NSBezierPath(ovalIn: CGRect(x: c.x + 18, y: c.y + 26, width: 34, height: 34)).fill()
NSGraphicsContext.restoreGraphicsState()
// Eye outline.
eye.lineWidth = 14; rgb(0x0E1433).setStroke(); eye.stroke()

NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
