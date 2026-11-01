"""Corpus integrity regression tests; no product compiler is invoked."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('corpus_validator', Path(__file__).with_name('validate.py'))
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class CorpusValidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / 'corpus'
        shutil.copytree(validator.ROOT, self.root, ignore=shutil.ignore_patterns('__pycache__'))
        self.manifest = json.loads((self.root / 'manifest.json').read_text())

    def write(self):
        (self.root / 'manifest.json').write_text(json.dumps(self.manifest))

    def test_real_corpus_and_request_include_all_sources(self):
        manifest, count = validator.validate(self.root)
        self.assertEqual((len(manifest['cases']), count), (14, 25))
        case = next(c for c in manifest['cases'] if c['id'] == 'included-file')
        request = validator.compile_request(case, 7, self.root)
        self.assertEqual(request['payload']['revision'], 7)
        self.assertEqual([d['path'] for d in request['payload']['documents']], ['main.tex', 'parts/section.tex'])
        self.assertEqual(request['payload']['documents'][1]['text'], 'Included café.\n')

    def test_witness_cannot_silently_drift(self):
        self.manifest['cases'][0]['expectations'][0]['source']['text'] = 'Changed'
        self.write()
        with self.assertRaises(ValueError):
            validator.validate(self.root)

    def test_multibyte_boundary_is_checked(self):
        case = next(c for c in self.manifest['cases'] if c['id'] == 'literal-source-map')
        case['expectations'][1]['source']['start_byte'] += 1
        self.write()
        with self.assertRaises(UnicodeError):
            validator.validate(self.root)

    def test_traversal_rejected(self):
        self.manifest['cases'][0]['documents'][0] = '../main.tex'
        self.write()
        with self.assertRaises(ValueError):
            validator.validate(self.root)

    def test_undeclared_fixture_rejected(self):
        (self.root / 'cases' / 'plain-paragraphs' / 'forgotten.tex').write_text('Unlisted')
        with self.assertRaises(ValueError):
            validator.validate(self.root)

    def test_support_claim_needs_separate_engine_evidence(self):
        self.manifest['cases'][0]['support'] = 'supported'
        self.write()
        with self.assertRaises(ValueError):
            validator.validate(self.root)


if __name__ == '__main__':
    unittest.main()
