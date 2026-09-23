"""Minimal OpenType reader: cmap, hmtx advances and MATH glyph assemblies.

Enough of the format to answer "does the bundled face carry this character,
at what advance, and is it a whole glyph or an assembly of parts?" -- which is
what decides whether a TeX math command can be painted at pdflatex's metrics.
Python 3 standard library only.
"""
import struct


class OpenType:
    def __init__(self, path):
        self.path = path
        with open(path, "rb") as fh:
            self.d = fh.read()
        n = struct.unpack(">H", self.d[4:6])[0]
        self.tables = {}
        for i in range(n):
            o = 12 + 16 * i
            tag = self.d[o:o + 4].decode("latin1")
            off, ln = struct.unpack(">II", self.d[o + 8:o + 16])
            self.tables[tag] = (off, ln)
        self._cmap = None
        hd = self.tables["head"][0]
        self.upm = self.u16(hd + 18)

    def u16(self, o):
        return struct.unpack(">H", self.d[o:o + 2])[0]

    def u32(self, o):
        return struct.unpack(">I", self.d[o:o + 4])[0]

    # -- cmap -------------------------------------------------------------
    def cmap(self):
        if self._cmap is not None:
            return self._cmap
        off = self.tables["cmap"][0]
        best = None
        for i in range(self.u16(off + 2)):
            p = off + 4 + 8 * i
            pid, eid = self.u16(p), self.u16(p + 2)
            sub = off + self.u32(p + 4)
            if (pid, eid) in ((3, 1), (3, 10), (0, 3), (0, 4), (0, 6)):
                fmt = self.u16(sub)
                if best is None or fmt == 12:
                    best = (sub, fmt)
        m = {}
        if best is None:
            self._cmap = m
            return m
        sub, fmt = best
        if fmt == 4:
            segx2 = self.u16(sub + 6)
            ends, starts = sub + 14, sub + 14 + segx2 + 2
            deltas, ranges = starts + segx2, starts + 2 * segx2
            for s in range(segx2 // 2):
                e, st = self.u16(ends + 2 * s), self.u16(starts + 2 * s)
                dl = struct.unpack(">h", self.d[deltas + 2 * s:deltas + 2 * s + 2])[0]
                ro = self.u16(ranges + 2 * s)
                for c in range(st, min(e, 0xFFFF) + 1):
                    if ro == 0:
                        g = (c + dl) & 0xFFFF
                    else:
                        g = self.u16(ranges + 2 * s + ro + 2 * (c - st))
                        if g:
                            g = (g + dl) & 0xFFFF
                    if g:
                        m[c] = g
        elif fmt == 12:
            for i in range(self.u32(sub + 12)):
                p = sub + 16 + 12 * i
                a, b, g = struct.unpack(">III", self.d[p:p + 12])
                for c in range(a, b + 1):
                    m[c] = g + (c - a)
        self._cmap = m
        return m

    # -- advances ---------------------------------------------------------
    def advance(self, gid):
        """Advance width in font units."""
        nh = self.u16(self.tables["hhea"][0] + 34)
        return self.u16(self.tables["hmtx"][0] + 4 * min(gid, nh - 1))

    def advance_pt(self, gid, size=10.0):
        return self.advance(gid) / self.upm * size

    # -- MATH assemblies --------------------------------------------------
    def coverage(self, o):
        fmt = self.u16(o)
        if fmt == 1:
            return [self.u16(o + 4 + 2 * i) for i in range(self.u16(o + 2))]
        gs = []
        for i in range(self.u16(o + 2)):
            p = o + 4 + 6 * i
            s, e, _ = struct.unpack(">HHH", self.d[p:p + 6])
            gs.extend(range(s, e + 1))
        return gs

    def assemblies(self):
        """-> (vertical, horizontal); each {gid: [(partGid, flags, fullAdv)]}."""
        if "MATH" not in self.tables:
            return {}, {}
        off = self.tables["MATH"][0]
        vo = self.u16(off + 8)
        if not vo:
            return {}, {}
        V = off + vo
        vcount = self.u16(V + 6)
        out = []
        for which in (0, 1):
            cov = self.u16(V + 2 + 2 * which)
            cnt = self.u16(V + 6 + 2 * which)
            base = V + 10 + (0 if which == 0 else 2 * vcount)
            gids = self.coverage(V + cov) if cov else []
            res = {}
            for i in range(min(cnt, len(gids))):
                co = self.u16(base + 2 * i)
                if not co:
                    continue
                C = V + co
                ao = self.u16(C)
                if not ao:
                    continue
                A = C + ao
                parts = []
                for j in range(self.u16(A + 4)):
                    p = A + 6 + 10 * j
                    g, _sc, _ec, fa, fl = struct.unpack(">HHHHH", self.d[p:p + 10])
                    parts.append((g, fl, fa))
                res[gids[i]] = parts
            out.append(res)
        return out[0], out[1]
