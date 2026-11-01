// P5-KEYSTROKE-MAIN: a plain NSTextView (no FlashTeX code) invalidates the display
// from the edited line to the end of the document on every keystroke.
// usage: swiftc -O nstextview-tail.swift -o /tmp/tvtail && /tmp/tvtail plain-10.tex
// Prints, per configuration, the tallest rect passed to setNeedsDisplay per edit.

import AppKit
final class TV: NSTextView {
    var log: [NSRect] = []
    override func setNeedsDisplay(_ rect: NSRect, avoidAdditionalLayout flag: Bool) { log.append(rect); super.setNeedsDisplay(rect, avoidAdditionalLayout: flag) }
}
let app = NSApplication.shared
let text = try! String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
for (wrap, noncontig, pstyle) in [(false,false,false),(false,true,false),(true,false,false),(true,true,false),(false,false,true),(true,false,true)] {
    let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 380, height: 670), styleMask: [.titled], backing: .buffered, defer: false)
    let scroll = TV.scrollableTextView()
    let tv0 = TV(frame: scroll.contentView.bounds)
    let tv = tv0
    tv.isVerticallyResizable = true
    scroll.documentView = tv
    tv.font = NSFont(name: "Menlo", size: 16)
    tv.layoutManager?.allowsNonContiguousLayout = noncontig
    if wrap {
        tv.textContainer?.widthTracksTextView = true; tv.isHorizontallyResizable = false; tv.autoresizingMask = [.width]
        tv.textContainer?.containerSize = NSSize(width: 380, height: CGFloat.greatestFiniteMagnitude)
    } else {
        tv.textContainer?.widthTracksTextView = false
        tv.textContainer?.containerSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        tv.isHorizontallyResizable = true; tv.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
    }
    w.contentView = scroll
    scroll.frame = w.contentView!.bounds
    tv.string = text
    if pstyle { let p = NSMutableParagraphStyle(); p.lineHeightMultiple = 1.2; tv.textStorage?.addAttribute(.paragraphStyle, value: p, range: NSRange(location: 0, length: (text as NSString).length)); tv.typingAttributes[.paragraphStyle] = p }
    tv.layoutManager?.ensureLayout(for: tv.textContainer!)
    tv.setSelectedRange(NSRange(location: 463, length: 0))
    tv.scrollRangeToVisible(NSRange(location: 463, length: 0))
    w.displayIfNeeded()
    var out: [String] = []
    for i in 0..<4 {
        tv.log = []
        if i % 2 == 0 { tv.insertText("x", replacementRange: tv.selectedRange()) } else { tv.deleteBackward(nil) }
        w.displayIfNeeded()
        let h = min(tv.log.map { $0.height }.max() ?? 0, 1e6)
        let tot = tv.log.reduce(0) { $0 + min($1.height, 1e6) }
        out.append("max h \(Int(h)) sum \(Int(tot)) n \(tv.log.count)")
    }
    print("wrap \(wrap) noncontig \(noncontig) pstyle \(pstyle) frame \(tv.frame.size): " + out.joined(separator: " | "))
}
