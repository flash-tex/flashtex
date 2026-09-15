// Draws a paragraph the way TeX sees it: source, tokens, boxes, glue,
// justified lines, page. Positions come from measuring the real fonts.
(function () {
  "use strict";

  var PROBLEM = [
    ["\\textbf{Problem", "<b>Problem</b>"], ["1.}", "<b>1.</b>"],
    ["Let", "Let"], ["$a,b$", "<i>a</i>, <i>b</i>"], ["be", "be"], ["positive", "positive"],
    ["integers.", "integers."], ["Suppose", "Suppose"], ["that", "that"], ["$a$", "<i>a</i>"],
    ["divides", "divides"], ["$b$.", "<i>b</i>."], ["Prove", "Prove"], ["that", "that"],
    ["$a$", "<i>a</i>"], ["divides", "divides"],
    ["$a^2+b^2$.", "<i>a</i><sup>2</sup>&#8201;+&#8201;<i>b</i><sup>2</sup>."]
  ];
  var HERO = PROBLEM.slice(2).concat([
    ["Is", "Is"], ["the", "the"], ["converse", "converse"], ["true?", "true?"], ["In", "In"],
    ["other", "other"], ["words,", "words,"], ["if", "if"], ["$a$", "<i>a</i>"], ["divides", "divides"],
    ["$a^2+b^2$,", "<i>a</i><sup>2</sup>&#8201;+&#8201;<i>b</i><sup>2</sup>,"], ["must", "must"],
    ["$a$", "<i>a</i>"], ["divide", "divide"], ["$b$?", "<i>b</i>?"], ["Prove", "Prove"],
    ["your", "your"], ["answer.", "answer."]
  ]);

  // Visual layers per state: source, token chips, boxes, width labels, glue, sheet.
  var LAYERS = {
    src:   [1, 1, 0, 0, 0, 0],
    chip:  [0, 1, 0, 0, 0, 0],
    box:   [0, 0, 1, 0.7, 0.35, 0],
    lab:   [0, 0, 1, 0, 0, 0],
    glue:  [0, 0, 0, 1, 1, 0],
    sheet: [0, 0, 0, 0, 0.5, 1]
  };
  var LAST = 5;

  function clamp(v, lo, hi) { return v < lo ? lo : v > hi ? hi : v; }
  function lerp(a, b, t) { return a + (b - a) * t; }
  function ease(t) { return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2; }

  function highlight(src) {
    return src.replace(/&/g, "&amp;").replace(/</g, "&lt;")
      .replace(/(\\[A-Za-z]+)|(\$)|([{}^])/g, function (m, cs, dollar) {
        if (cs) return '<span class="t-cs">' + cs + "</span>";
        return '<span class="' + (dollar ? "t-math" : "t-grp") + '">' + m + "</span>";
      });
  }

  function el(tag, cls, parent) {
    var n = document.createElement(tag);
    n.className = cls;
    if (parent) parent.appendChild(n);
    return n;
  }

  function Galley(root, units, only) {
    this.root = root;
    this.units = units;
    this.only = only;
    this.layer = el("div", "galley-layer", root);
    this.layer.setAttribute("aria-hidden", "true");
    this.sheet = el("div", "g-sheet", this.layer);
    this.glue = [];
    this.nodes = [];
    for (var i = 0; i < units.length; i++) {
      if (i) this.glue.push(el("div", "g-glue", this.layer));
      var n = el("div", "g-unit", this.layer);
      n.innerHTML = '<span class="g-src">' + highlight(units[i][0]) + '</span><span class="g-out">' +
        units[i][1] + '</span><span class="g-w"></span>';
      this.nodes.push(n);
    }
    root.classList.add("is-live");
    this.p = 0;
  }

  Galley.prototype.layout = function () {
    var W = this.root.clientWidth;
    if (!W) return;
    var small = W < 520;
    var serif = small ? 17 : 21, mono = small ? 12.5 : 14.5;
    this.root.style.setProperty("--g-serif", serif + "px");
    this.root.style.setProperty("--g-mono", mono + "px");

    var ws = [], wo = [], labelled = [];
    for (var i = 0; i < this.nodes.length; i++) {
      var n = this.nodes[i];
      ws.push(n.children[0].getBoundingClientRect().width);
      wo.push(n.children[1].getBoundingClientRect().width);
      var label = (wo[i] / serif * 10).toFixed(2) + "pt";
      n.children[2].textContent = label;
      // Leave room for the width label when it is wider than its box.
      labelled.push(small ? wo[i] : Math.max(wo[i], label.length * 6.2 + 4));
    }

    var top = small ? 18 : 22;
    function flow(widths, gap, lineH, width, x0, y0, advance) {
      var x = 0, y = 0, out = [];
      advance = advance || widths;
      for (var k = 0; k < widths.length; k++) {
        if (k) {
          if (x + gap + advance[k] > width) { x = 0; y += lineH; } else { x += gap; }
        }
        out.push({ x: x0 + x, y: y0 + y, w: widths[k] });
        x += advance[k];
      }
      return out;
    }

    var hs = mono * 1.7, ho = serif * 1.3;
    var states = [
      { pos: flow(ws, mono * 0.6, mono * 2.2, W, 0, top), h: hs },
      { pos: flow(ws, 16, mono * 3.3, W, 5, top), h: hs },
      { pos: flow(wo, small ? 12 : 14, serif * 2.9, W, 0, top, labelled), h: ho },
      { pos: flow(wo, serif * 0.9, serif * 2.1, W, 0, top), h: ho }
    ];

    // Justified lines: break on natural interword space, then stretch the glue.
    var pad = small ? 20 : 40;
    var M = Math.min(W - pad * 2, serif * 24);
    var x0 = (W - M) / 2, lineH = serif * 1.62;
    var natural = serif / 3;
    var lines = [], cur = [], curW = 0;
    for (var j = 0; j < wo.length; j++) {
      var add = cur.length ? natural + wo[j] : wo[j];
      if (cur.length && curW + add > M) { lines.push(cur); cur = [j]; curW = wo[j]; }
      else { cur.push(j); curW += add; }
    }
    if (cur.length) lines.push(cur);
    var just = [];
    lines.forEach(function (line, li) {
      var sum = line.reduce(function (s, k) { return s + wo[k]; }, 0);
      var last = li === lines.length - 1;
      var gap = line.length > 1 && !last ? (M - sum) / (line.length - 1) : natural;
      var x = x0;
      line.forEach(function (k) {
        just[k] = { x: x, y: top + pad + li * lineH, w: wo[k] };
        x += wo[k] + gap;
      });
    });
    states.push({ pos: just, h: ho });
    states.push({ pos: just, h: ho });
    this.sheetRect = { x: x0 - pad, y: top, w: M + pad * 2, h: lines.length * lineH + pad * 2 - (lineH - ho) };

    var height = 0;
    if (this.only === undefined) {
      states.forEach(function (s) {
        s.pos.forEach(function (q) { height = Math.max(height, q.y + s.h); });
      });
    }
    height = Math.max(height + 12, this.sheetRect.y + this.sheetRect.h + 12);
    if (this.only !== undefined) {
      // A single fixed state: trim the layer to the sheet itself.
      height = this.sheetRect.h;
      this.layer.style.top = -this.sheetRect.y + "px";
    }
    this.layer.style.height = height + "px";
    var sh = this.sheet.style;
    sh.transform = "translate(" + this.sheetRect.x + "px," + this.sheetRect.y + "px)";
    sh.width = this.sheetRect.w + "px";
    sh.height = this.sheetRect.h + "px";

    this.states = states;
    this.render(this.p);
  };

  Galley.prototype.render = function (p) {
    this.p = p;
    if (!this.states) return;
    var i = Math.min(Math.floor(p), LAST - 1);
    // Hold each state for a while so it reads, then move quickly to the next.
    var t = ease(clamp((p - i - 0.22) / 0.56, 0, 1));
    var A = this.states[i], B = this.states[i + 1];
    var h = lerp(A.h, B.h, t);
    var cur = [];
    for (var k = 0; k < this.nodes.length; k++) {
      var a = A.pos[k], b = B.pos[k];
      var q = { x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t), w: lerp(a.w, b.w, t) };
      cur.push(q);
      var s = this.nodes[k].style;
      s.transform = "translate3d(" + q.x.toFixed(2) + "px," + q.y.toFixed(2) + "px,0)";
      s.width = q.w.toFixed(2) + "px";
      s.height = h.toFixed(2) + "px";
    }
    for (var g = 0; g < this.glue.length; g++) {
      var l = cur[g], r = cur[g + 1];
      var gx = l.x + l.w, gw = r.x - gx;
      var show = Math.abs(l.y - r.y) < 3 && gw > 2;
      this.glue[g].hidden = !show;
      if (show) {
        var gs = this.glue[g].style;
        gs.transform = "translate3d(" + gx.toFixed(2) + "px," + l.y.toFixed(2) + "px,0)";
        gs.width = gw.toFixed(2) + "px";
        gs.height = h.toFixed(2) + "px";
      }
    }
    for (var key in LAYERS) {
      this.root.style.setProperty("--" + key, lerp(LAYERS[key][i], LAYERS[key][i + 1], t).toFixed(3));
    }
  };

  function whenFontsReady(fn) {
    if (!document.fonts || !document.fonts.load) { fn(); return; }
    Promise.all([
      document.fonts.load('21px "Latin Modern Roman"'),
      document.fonts.load('italic 21px "Latin Modern Roman"'),
      document.fonts.load('700 21px "Latin Modern Roman"'),
      document.fonts.load('14px "Fragment Mono"')
    ]).then(fn, fn);
  }

  function observe(galley) {
    if ("ResizeObserver" in window) {
      var lastW = 0;
      new ResizeObserver(function () {
        var w = galley.root.clientWidth;
        if (w !== lastW) { lastW = w; galley.layout(); }
      }).observe(galley.root);
    } else {
      window.addEventListener("resize", function () { galley.layout(); });
    }
  }

  var params = new URLSearchParams(location.search);
  var pinned = params.has("state") ? clamp(parseFloat(params.get("state")) || 0, 0, LAST) : null;
  var still = pinned !== null || window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // Hero: the finished paragraph with its boxes and glue left visible.
  var heroRoot = document.querySelector('[data-galley="hero"]');
  if (heroRoot) {
    var hero = new Galley(heroRoot, HERO, 4);
    hero.render = (function (base) {
      return function () {
        base.call(this, 4);
        var s = this.root.style;
        s.setProperty("--box", "0.9"); s.setProperty("--glue", "1"); s.setProperty("--sheet", "1");
      };
    })(hero.render);
    whenFontsReady(function () { hero.layout(); observe(hero); });
  }

  // The pinned sequence: scroll position scrubs through the six states.
  var scrolly = document.querySelector("[data-scrolly]");
  if (!scrolly) return;
  var stageRoot = scrolly.querySelector('[data-galley="problem"]');
  var steps = Array.prototype.slice.call(scrolly.querySelectorAll(".step"));
  var galley = new Galley(stageRoot, PROBLEM);
  if (still) {
    document.documentElement.classList.remove("js-motion");
    document.documentElement.classList.add("js-still");
  }
  var stillP = pinned !== null ? pinned : LAST;

  function progress() {
    var r = scrolly.getBoundingClientRect();
    var total = r.height - window.innerHeight;
    return total > 0 ? clamp(-r.top / total, 0, 1) : 1;
  }
  function toP(raw) { return clamp(raw * 1.12 - 0.04, 0, 1) * LAST; }

  function setActive(p) {
    var active = Math.round(p);
    steps.forEach(function (s, k) {
      var on = k === active;
      s.classList.toggle("is-active", on);
      var btn = s.querySelector(".step-btn");
      if (btn) btn.setAttribute("aria-current", on ? "step" : "false");
    });
  }

  var ticking = false;
  function frame() {
    ticking = false;
    var p = still ? stillP : toP(progress());
    galley.render(p);
    setActive(p);
  }
  function request() {
    if (!ticking) { ticking = true; requestAnimationFrame(frame); }
  }

  steps.forEach(function (s, k) {
    var btn = s.querySelector(".step-btn");
    if (!btn) return;
    btn.addEventListener("click", function () {
      if (still) {
        stillP = k;
        request();
        var r0 = stageRoot.getBoundingClientRect();
        if (r0.bottom > window.innerHeight || r0.top < 0) stageRoot.scrollIntoView({ block: "nearest" });
        return;
      }
      var r = scrolly.getBoundingClientRect();
      var total = r.height - window.innerHeight;
      var raw = (k / LAST + 0.04) / 1.12;
      window.scrollTo({ top: window.scrollY + r.top + raw * total + 1, behavior: "smooth" });
    });
  });

  whenFontsReady(function () {
    galley.layout();
    observe(galley);
    frame();
    if (!still) {
      window.addEventListener("scroll", request, { passive: true });
      window.addEventListener("resize", request);
    }
  });
})();
