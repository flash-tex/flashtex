import Foundation

extension TextDocument {
    /// Check the tree and the indexes against a recomputation: subtree sums,
    /// parent and leaf links, the run index, run sizes. Nil if consistent.
    /// For tests; O(n).
    public func validate() -> String? {
        func sums(_ n: Node) -> (Int, Int, Int, Int)? {
            var r = 0, v = 0, a = 0, b = 0
            if n.isLeaf {
                for it in n.items {
                    if it.leaf !== n { return nil }
                    if it.len < 1 || it.len > CollabLimits.maxRunScalars { return nil }
                    if UTF8Scan.scalarCount(it.bytes) != it.len || UTF8Scan.utf16Count(it.bytes) != it.u16 { return nil }
                    r += it.len; v += it.visLen; a += it.visU16; b += it.visU8
                }
            } else {
                for c in n.children {
                    if c.parent !== n { return nil }
                    guard let s = sums(c) else { return nil }
                    r += s.0; v += s.1; a += s.2; b += s.3
                }
            }
            guard r == n.raw, v == n.vis, a == n.vu16, b == n.vu8 else { return nil }
            return (r, v, a, b)
        }
        if sums(root) == nil { return "subtree sums, run sizes or parent links are wrong" }
        var count = 0
        var leaf: Node? = firstLeaf
        var last: Node? = nil
        while let l = leaf {
            if l.prev !== last { return "leaf chain" }
            for it in l.items {
                count += 1
                guard let (found, off) = lookup(it.firstID), found === it, off == 0 else {
                    return "run index misses \(it.firstID)"
                }
            }
            last = l
            leaf = l.next
        }
        if last !== lastLeaf { return "last leaf" }
        if count != runCount { return "run index has \(runCount) runs, tree \(count)" }
        return nil
    }
}
