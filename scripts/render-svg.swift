// Renders an SVG to a PNG with a transparent background (qlmanage fills white).
// Usage: swift scripts/render-svg.swift in.svg out.png 1024
// Exits 1 if the top-left pixel is not transparent, so a white-cornered icon can't ship.
import AppKit

let a = CommandLine.arguments
let size = Int(a[3])!
guard let svg = NSImage(contentsOf: URL(fileURLWithPath: a[1])) else { fatalError("cannot read \(a[1])") }
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8,
                           samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                           colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
NSColor.clear.set()
NSRect(x: 0, y: 0, width: size, height: size).fill()
svg.draw(in: NSRect(x: 0, y: 0, width: size, height: size))
NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: a[2]))
if rep.colorAt(x: 0, y: 0)!.alphaComponent > 0.01 { print("corner is not transparent"); exit(1) }
