#!/usr/bin/env python3
"""Self-tests of gen-supported-latex.py (no TeX needed). MIT.

    python3 -m unittest discover -s apps/mac/scripts -p 'test_gen_supported_latex.py'
"""
import importlib.util
import io
import json
import os
import contextlib
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("gen_supported_latex", os.path.join(HERE, "gen-supported-latex.py"))
gen = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gen)


class SmokeLog(unittest.TestCase):
    LOG = ("amsmath          equal\n"
           "tabu             EXCLUDED (the reference does not compile it on this TeX Live; "
           "the candidate fails identically: exit 1)\n"
           "xkeyval-smoke    DIFFERENT (pass 1: transcript differs)\n"
           "3 documents, 1 differ, 1 excluded (the reference does not compile it)\n")

    def test_verdicts_and_reasons(self):
        verdicts, total = gen.parse_smoke_log(self.LOG)
        self.assertEqual(total, 3)
        self.assertEqual(verdicts["amsmath"], ("equal", ""))
        self.assertEqual(verdicts["tabu"][0], "excluded")
        self.assertIn("fails identically", verdicts["tabu"][1])
        self.assertEqual(verdicts["xkeyval-smoke"], ("different", "pass 1: transcript differs"))

    def test_a_partial_log_is_refused(self):
        with self.assertRaises(gen.Usage):
            gen.parse_smoke_log("amsmath          equal\n")  # no summary line
        with self.assertRaises(gen.Usage):
            gen.parse_smoke_log("amsmath          equal\n2 documents, 0 differ\n")

    def test_only_equal_documents_are_verified(self):
        docs = {"amsmath": {"package": "amsmath", "file": "amsmath.tex"},
                "tabu": {"package": "tabu", "file": "tabu.tex"},
                "xkeyval-smoke": {"package": "xkeyval", "file": "xkeyval-smoke.tex"}}
        measured = {"documents": {"amsmath": "equal", "tabu": "excluded: x", "xkeyval-smoke": "different: y"}}
        self.assertEqual(gen.verified_packages(measured, docs), {"amsmath": "amsmath"})
        freq = {"amsmath": 346, "tabu": 400}
        self.assertEqual(gen.vocabulary_packages(measured, docs, freq), ["amsmath"])
        self.assertEqual(gen.vocabulary_packages(measured, docs, {"amsmath": gen.VOCABULARY_MIN_DOCUMENTS - 1}), [])


class Candidates(unittest.TestCase):
    def test_internal_names_are_whole_and_dropped(self):
        names, envs = gen.candidates(r"\def\foo@bar{}\baz \l_tmpa_tl \endquux \newenvironment{wide*}{}{}"
                                     r"\NewDocumentEnvironment{boxy}{m}{}{}")
        self.assertEqual(names, {"def", "baz", "endquux", "newenvironment", "NewDocumentEnvironment"})
        self.assertEqual(envs, {"quux", "wide*", "boxy"})

    def test_the_documents_own_options(self):
        doc = "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage{amsmath,amssymb}\n"
        self.assertEqual(gen.usepackage_line("fontenc", doc), "\\usepackage[T1]{fontenc}")
        self.assertEqual(gen.usepackage_line("amssymb", doc), "\\usepackage{amssymb}")
        self.assertEqual(gen.usepackage_line("bm", doc), "\\usepackage{bm}")


class ProbeOutput(unittest.TestCase):
    def test_lines(self):
        out = "\n".join([
            "p frac",
            "q center",
            "d dfrac macro:->\\genfrac {}{}{}0",
            "s boldsymbol macro:#1->\\mathchoice {}",
            "e align \\long macro:->\\endalign@",
            "e input \\endinput",            # a primitive: not an environment end
            "a foo m o",
            "a bar \\relax ",                 # \GetDocumentCommandArgSpec refused it
        ])
        pre, pre_env, defs, envs = gen.parse_probe_output(out)
        self.assertEqual(pre, {"frac"})
        self.assertEqual(pre_env, {"center"})
        self.assertEqual(envs, {"align"})
        self.assertEqual(defs["dfrac"], {"d": "macro:->\\genfrac {}{}{}0"})
        self.assertEqual(defs["foo"], {"a": "m o"})
        self.assertNotIn("bar", defs)

    def test_the_probe_document_loads_between_pre_and_post(self):
        tex = gen.probe_document("\\usepackage{bm}", ({"bm"}, set()), ({"bm", "boldsymbol"}, {"x"}), ["bm"])
        load = tex.index("\\usepackage{bm}")
        self.assertLess(tex.index("\\ft@pre{bm}"), load)
        self.assertGreater(tex.index("\\ft@post{boldsymbol}"), load)
        self.assertGreater(tex.index("\\ft@spec{bm}"), tex.index("\\ft@postenv{x}"))
        self.assertTrue(tex.startswith("\\documentclass{article}"))

    def test_prune_keeps_only_what_classify_reads(self):
        defs = {
            "plain": {"d": "macro:#1->x", "s": "unused", "b": "unused"},
            "robust": {"d": "macro:->\\protect \\robust  ", "s": "macro:->\\@protected@testopt \\robust  \\\\robust  {}",
                       "c": "macro:[#1]#2->", "b": "unused"},
            "opt": {"d": "macro:->\\@protected@testopt \\opt \\\\opt {}", "b": "macro:[#1]#2#3->"},
            "long": {"d": "macro:->" + "x" * 500},
        }
        p = gen.prune(defs)
        self.assertEqual(set(p["plain"]), {"d"})
        self.assertEqual(set(p["robust"]), {"d", "s", "c"})
        self.assertEqual(set(p["opt"]), {"d", "b"})
        self.assertEqual(len(p["long"]["d"]), gen.MEANING_CHARS)


class Classify(unittest.TestCase):
    def k(self, name, **d):
        return gen.classify(name, d)

    def test_symbols_and_math_only(self):
        self.assertEqual(self.k("alpha", d='\\mathchar"10B'), ("symbol", ""))
        self.assertEqual(self.k("hat", d='macro:->\\mathaccent "705E '), ("math", ""))
        self.assertEqual(self.k("over", d="\\over"), ("math", ""))
        self.assertEqual(self.k("lra", d="macro:->\\leftarrow \\rightarrow "), ("text", ""))
        self.assertEqual(self.k("boldsymbol", d="macro:->\\protect \\boldsymbol  ",
                                s="macro:#1->\\mathchoice {x}"), ("math", "{#1}"))

    def test_primitives_and_others(self):
        self.assertEqual(self.k("hbox", d="\\hbox"), ("primitive", ""))
        self.assertEqual(self.k("bgroup", d="begin-group character {"), ("text", ""))
        self.assertEqual(self.k("endgraf", d="\\par"), ("text", ""))
        self.assertEqual(self.k("pdfoutput", d="\\relax"), (None, ""))
        self.assertEqual(self.k("relax", d="\\relax"), ("primitive", ""))

    def test_argument_shapes(self):
        self.assertEqual(self.k("frac", d="\\long macro:#1#2->{\\begingroup #1\\endgroup \\over #2}"),
                         ("text", "{#1}{#2}"))
        self.assertEqual(self.k("sqrtx", d="macro:->\\@protected@testopt \\sqrtx \\\\sqrtx {}",
                                b="macro:[#1]#2->"), ("text", "[#1]{#2}"))
        self.assertEqual(self.k("rob", d="macro:->\\protect \\rob  ", s="macro:->\\@protected@testopt \\rob  \\\\rob  {}",
                                c="macro:[#1]#2#3->"), ("text", "[#1]{#2}{#3}"))
        self.assertEqual(self.k("delim", d="macro:#1\\par ->"), ("text", ""))
        self.assertEqual(self.k("doc", d="\\protected macro:->\\__cmd_start:nNNnnn", a="s O{x} m"),
                         ("text", "*[#2]{#3}"))

    def test_spec_arguments(self):
        self.assertEqual(gen.spec_arguments("m o m"), "{#1}[#2]{#3}")
        self.assertEqual(gen.spec_arguments("+m >{\\SplitList{,}}m"), "{#1}{#2}")
        self.assertEqual(gen.spec_arguments("D(){0,0} m"), "{#2}")
        self.assertEqual(gen.spec_arguments("t+ m"), "+{#2}")
        self.assertEqual(gen.spec_arguments("?"), "")

    def test_a_text_command_is_offered_in_math_too(self):
        rows = gen.rows("amsmath", "\\usepackage{amsmath}", {
            "alpha": {"d": '\\mathchar"10B'},
            "text": {"d": "\\protected macro:->\\__cmd_start:nNNnnn", "a": "m"},
            "placeholder": {"d": "\\relax"},
        })
        got = [(r["name"], r["mode"], r["origin"], r["arguments"]) for r in rows]
        self.assertEqual(got, [("alpha", "math", "math_symbol", ""),
                               ("text", "text", "text_dispatch", "{#1}"),
                               ("text", "math", "math_structure", "{#1}")])
        self.assertTrue(all(r["renders"] for r in rows))


class Render(unittest.TestCase):
    MEASURED = {"engine_commit": "b" * 40, "source": "run", "oracle": {"version": gen.ORACLE_VERSION},
                "documents": {"amsmath": "equal", "tabu": "excluded: no reference"}}
    DOCS = {"amsmath": {"package": "amsmath", "file": "amsmath.tex"}, "tabu": {"package": "tabu", "file": "tabu.tex"}}
    PROBE = {"oracle": {"version": gen.ORACLE_VERSION},
             "kernel": {"commands": {"textbf": {"d": "macro:->\\protect \\textbf  ", "s": "\\long macro:#1->x"}},
                        "environments": ["center"]},
             "packages": {"amsmath": {"load": "\\usepackage{amsmath}",
                                      "commands": {"dfrac": {"d": "macro:->\\genfrac {}{}{}0"},
                                                   "textbf": {"d": "macro:#1->y"}},
                                      "environments": ["align", "center"]}}}

    def data(self):
        return gen.render_data(self.MEASURED, self.PROBE, self.DOCS, {"amsmath": 346, "tabu": 30}, 400)

    def test_the_editors_decode_it(self):
        d = json.loads(gen.dumps_supported(self.data()))
        self.assertEqual(d["schema"], "flashtex-supported-latex/1")
        for key in ("generator", "compiler_version"):
            self.assertIsInstance(d[key], str)
        for c in d["commands"]:  # LaTeXVocabulary.Inventory.Command's required fields
            for key, kind in (("name", str), ("mode", str), ("origin", str), ("arguments", str),
                              ("description", str), ("renders", bool)):
                self.assertIsInstance(c[key], kind)
            self.assertIn(c["mode"], ("text", "math"))
            self.assertIn(c["origin"], ("text_dispatch", "math_structure", "math_symbol", "expansion"))
        for e in d["environments"]:
            self.assertEqual(set(e) >= {"name", "mode", "description"}, True)

    def test_the_kernel_comes_first_and_a_name_once_per_mode(self):
        d = self.data()
        self.assertEqual([(c["name"], c["mode"], c["source"]) for c in d["commands"]],
                         [("textbf", "text", "latex"), ("textbf", "math", "latex"),
                          ("dfrac", "text", "amsmath"), ("dfrac", "math", "amsmath")])
        self.assertEqual([e["name"] for e in d["environments"]], ["center", "align"])

    def test_every_measured_package_is_listed_with_its_verdict(self):
        pk = {p["name"]: p for p in self.data()["packages"]}
        self.assertEqual(pk["amsmath"]["verdict"], "equal")
        self.assertTrue(pk["amsmath"]["vocabulary"])
        self.assertEqual((pk["tabu"]["verdict"], pk["tabu"]["reason"]), ("excluded", "no reference"))
        self.assertFalse(pk["tabu"]["vocabulary"])

    def test_one_row_per_line_and_deterministic(self):
        a, b = gen.dumps_supported(self.data()), gen.dumps_supported(self.data())
        self.assertEqual(a, b)
        self.assertIn('\n    {"name": "dfrac", "mode": "text"', a)


class Committed(unittest.TestCase):
    """The committed supported-latex.json is what render makes from the committed inputs."""

    def test_committed_file_is_current(self):
        if not os.path.exists(gen.PROBE):
            self.skipTest("no probe.json yet (the supported-latex workflow measures it)")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            status = gen.main(["check"])
        self.assertEqual(status, 0, buf.getvalue())


if __name__ == "__main__":
    unittest.main()
