"""Synthetic raster checks; no reference compiler or external renderer is invoked."""
from contextlib import redirect_stdout
import copy
import importlib.util
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
import zlib

from PIL import Image, PngImagePlugin

SPEC = importlib.util.spec_from_file_location('raster_compare', Path(__file__).with_name('compare.py'))
comparator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(comparator)


class RasterTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def save(self, name, pixels=None, size=(4, 3), mode='RGBA', **kwargs):
        path = self.root / name
        image = Image.new(mode, size, (255, 255, 255, 255) if mode == 'RGBA' else (255, 255, 255) if mode == 'RGB' else 0)
        for xy, color in (pixels or {}).items():
            image.putpixel(xy, color)
        image.save(path, **kwargs)
        return path

    def provenance(self, reference, candidate):
        result = {'schema_version': 1, 'case_id': 'synthetic', 'profile': 'synthetic-rgba8',
                  'source_sha256': 'a' * 64, 'page_index': 0}
        for role, image in (('reference', reference), ('candidate', candidate)):
            result[role] = {'image_sha256': image['image_sha256'], 'pixel_sha256': image['pixel_sha256'],
                            'source_sha256': 'a' * 64, 'rasterizer_binary_sha256': 'b' * 64,
                            'rasterizer_version_and_build': 'synthetic generator, no oracle',
                            'raster_argv': ['synthetic', 'INPUT.pdf', 'OUTPUT_PREFIX']}
        return result

    def pair(self, **candidate_kwargs):
        a = comparator.load_image(self.save('reference.png'))
        b = comparator.load_image(self.save('candidate.png', **candidate_kwargs))
        return a, b, self.provenance(a, b)

    def test_identical_pixels_pass_with_null_bbox_and_zero_counts(self):
        result = comparator.compare(*self.pair())
        self.assertEqual(result['status'], 'pass')
        self.assertTrue(result['exact_equal'])
        self.assertEqual(result['differing_pixel_count'], 0)
        self.assertEqual(result['max_channel_delta'], 0)
        self.assertIsNone(result['mismatch_bbox'])
        self.assertEqual(result['regions'], [])
        self.assertEqual(result['adjustments_applied'], [])

    def test_identical_pixels_different_png_encoding_pass(self):
        a = comparator.load_image(self.save('reference.png', compress_level=0))
        b = comparator.load_image(self.save('candidate.png', compress_level=9))
        self.assertNotEqual(a['image_sha256'], b['image_sha256'])
        self.assertEqual(a['pixel_sha256'], b['pixel_sha256'])
        self.assertTrue(comparator.compare(a, b, self.provenance(a, b))['exact_equal'])

    def test_one_pixel_one_channel_delta_always_fails(self):
        result = comparator.compare(*self.pair(pixels={(2, 1): (254, 255, 255, 255)}))
        self.assertEqual(result['status'], 'fail')
        self.assertFalse(result['exact_equal'])
        self.assertEqual(result['differing_pixel_count'], 1)
        self.assertEqual(result['max_channel_delta'], 1)
        self.assertEqual(result['mismatch_bbox'], [2, 1, 3, 2])
        self.assertEqual(result['differing_channel_counts'], {'red': 1, 'green': 0, 'blue': 0, 'alpha': 0})

    def test_alpha_only_difference_fails(self):
        result = comparator.compare(*self.pair(pixels={(0, 0): (255, 255, 255, 254)}))
        self.assertEqual(result['differing_channel_counts']['alpha'], 1)
        self.assertFalse(result['exact_equal'])

    def test_rgb_difference_under_zero_alpha_is_not_discarded(self):
        a = comparator.load_image(self.save('reference.png', pixels={(0, 0): (0, 0, 0, 0)}))
        b = comparator.load_image(self.save('candidate.png', pixels={(0, 0): (1, 0, 0, 0)}))
        self.assertFalse(comparator.compare(a, b, self.provenance(a, b))['exact_equal'])

    def test_rgb_only_adds_opaque_alpha(self):
        a = comparator.load_image(self.save('reference.png', mode='RGB'))
        b = comparator.load_image(self.save('candidate.png', mode='RGBA'))
        self.assertEqual(a['rgba'], b['rgba'])
        self.assertTrue(comparator.compare(a, b, self.provenance(a, b))['exact_equal'])

    def test_different_dimensions_fail_without_cropping_overlap(self):
        result = comparator.compare(*self.pair(size=(5, 3)))
        self.assertFalse(result['dimensions_equal'])
        self.assertFalse(result['exact_equal'])
        self.assertEqual(result['missing_pixel_count'], 3)
        self.assertIsNone(result['differing_pixel_count'])
        self.assertFalse(result['overlap_pixels_compared'])

    def test_equal_byte_counts_with_different_shape_fail(self):
        result = comparator.compare(*self.pair(size=(3, 4)))
        self.assertEqual(result['reference']['pixel_sha256'], result['candidate']['pixel_sha256'])
        self.assertFalse(result['exact_equal'])

    def test_regions_and_bbox_are_deterministic_and_truncation_does_not_relax(self):
        a = comparator.load_image(self.save('reference.png', size=(130, 70)))
        b = comparator.load_image(self.save('candidate.png', size=(130, 70), pixels={
            (0, 0): (0, 0, 0, 255), (63, 1): (0, 0, 0, 255),
            (64, 2): (0, 0, 0, 255), (129, 69): (0, 0, 0, 255)}))
        provenance = self.provenance(a, b)
        full = comparator.compare(a, b, provenance)
        brief = comparator.compare(a, b, provenance, max_regions=1)
        self.assertEqual(full['mismatch_bbox'], [0, 0, 130, 70])
        self.assertEqual(full['region_count'], 3)
        self.assertEqual(full['regions'][0]['bbox'], [0, 0, 64, 2])
        self.assertEqual(brief['differing_pixel_count'], 4)
        self.assertEqual(brief['region_count'], 3)
        self.assertTrue(brief['regions_truncated'])
        self.assertFalse(brief['exact_equal'])
        self.assertEqual(full, comparator.compare(a, b, provenance))

    def test_regrouping_regions_cannot_hide_changed_pixel(self):
        args = self.pair(pixels={(2, 1): (0, 255, 255, 255)})
        for size in (1, 2, 64, 1000):
            self.assertFalse(comparator.compare(*args, region_size=size)['exact_equal'])

    def test_identical_image_with_wrong_hash_fails_provenance(self):
        a, b, provenance = self.pair()
        provenance['candidate']['image_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'image SHA256'):
            comparator.compare(a, b, provenance)

    def test_source_pixel_and_rasterizer_bindings_are_checked(self):
        mutations = (
            ('source_sha256', 'c' * 64), ('pixel_sha256', 'c' * 64),
            ('rasterizer_binary_sha256', 'c' * 64),
            ('rasterizer_version_and_build', 'different renderer'),
            ('raster_argv', ['renderer', '--blur']))
        for key, value in mutations:
            with self.subTest(field=key):
                a, b, provenance = self.pair()
                provenance['candidate'][key] = value
                with self.assertRaises(ValueError):
                    comparator.compare(a, b, provenance)

    def test_required_fields_cannot_be_omitted(self):
        for field in ('image_sha256', 'source_sha256', 'rasterizer_binary_sha256', 'rasterizer_version_and_build', 'raster_argv'):
            a, b, provenance = self.pair()
            del provenance['candidate'][field]
            with self.assertRaises(ValueError):
                comparator.compare(a, b, provenance)

    def test_provenance_top_level_fields_are_validated(self):
        cases = (
            ('schema_version', 0), ('schema_version', 2), ('schema_version', '1'),
            ('schema_version', True), ('schema_version', None),
            ('case_id', ''), ('case_id', '   '), ('case_id', 123), ('case_id', None),
            ('profile', ''), ('profile', '   '), ('profile', 123), ('profile', None),
            ('source_sha256', 'xyz'), ('source_sha256', 'A' * 64),
            ('source_sha256', 'a' * 63), ('source_sha256', 123), ('source_sha256', None),
            ('page_index', -1), ('page_index', 1.0), ('page_index', True),
            ('page_index', '0'), ('page_index', None),
        )
        for field, value in cases:
            with self.subTest(field=field, value=value):
                a, b, provenance = self.pair()
                provenance[field] = value
                with self.assertRaises(ValueError):
                    comparator.compare(a, b, provenance)
        for field in ('schema_version', 'case_id', 'profile', 'source_sha256', 'page_index'):
            with self.subTest(field=field, value='<missing>'):
                a, b, provenance = self.pair()
                del provenance[field]
                with self.assertRaises(ValueError):
                    comparator.compare(a, b, provenance)
        for provenance in (None, [], 'provenance'):
            with self.subTest(provenance=provenance):
                a, b, _ = self.pair()
                with self.assertRaises(ValueError):
                    comparator.compare(a, b, provenance)

    def test_provenance_role_records_must_be_objects(self):
        for role in ('reference', 'candidate'):
            for value in (None, [], 'record'):
                with self.subTest(role=role, value=value):
                    a, b, provenance = self.pair()
                    provenance[role] = value
                    with self.assertRaisesRegex(ValueError, role + ' object'):
                        comparator.compare(a, b, provenance)
            with self.subTest(role=role, value='<missing>'):
                a, b, provenance = self.pair()
                del provenance[role]
                with self.assertRaisesRegex(ValueError, role + ' object'):
                    comparator.compare(a, b, provenance)

    def test_malformed_rasterizer_bindings_rejected_for_both_roles(self):
        cases = (
            ('rasterizer_binary_sha256', 'xyz', 'needs rasterizer_binary_sha256'),
            ('rasterizer_binary_sha256', 'B' * 64, 'needs rasterizer_binary_sha256'),
            ('rasterizer_binary_sha256', '', 'needs rasterizer_binary_sha256'),
            ('rasterizer_version_and_build', '', 'needs rasterizer_version_and_build'),
            ('rasterizer_version_and_build', '   ', 'needs rasterizer_version_and_build'),
            ('rasterizer_version_and_build', 123, 'needs rasterizer_version_and_build'),
            ('raster_argv', [], 'needs nonempty raster_argv'),
            ('raster_argv', [''], 'needs nonempty raster_argv'),
            ('raster_argv', ['render', 0], 'needs nonempty raster_argv'),
            ('raster_argv', 'render', 'needs nonempty raster_argv'),
            ('raster_argv', None, 'needs nonempty raster_argv'),
        )
        for role in ('reference', 'candidate'):
            for field, value, message in cases:
                with self.subTest(role=role, field=field, value=value):
                    a, b, provenance = self.pair()
                    provenance[role][field] = value
                    with self.assertRaisesRegex(ValueError, message):
                        comparator.compare(a, b, provenance)

    def test_pixel_hash_is_optional_but_verified_when_supplied(self):
        a, b, provenance = self.pair()
        del provenance['reference']['pixel_sha256']
        del provenance['candidate']['pixel_sha256']
        self.assertTrue(comparator.compare(a, b, provenance)['exact_equal'])
        a, b, provenance = self.pair()
        del provenance['reference']['pixel_sha256']
        self.assertTrue(comparator.compare(a, b, provenance)['exact_equal'])
        a, b, provenance = self.pair()
        provenance['candidate']['pixel_sha256'] = 'c' * 64
        with self.assertRaisesRegex(ValueError, 'pixel SHA256'):
            comparator.compare(a, b, provenance)

    def test_palette_grayscale_and_non_png_rejected(self):
        for name, mode in (('palette.png', 'P'), ('gray.png', 'L'), ('rgb.jpg', 'RGB')):
            with self.subTest(name=name):
                path = self.save(name, mode=mode)
                with self.assertRaises(ValueError):
                    comparator.load_image(path)

    def test_sixteen_bit_rgb_cannot_silently_reduce_precision(self):
        def chunk(kind, data):
            return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
        encoded = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 16, 2, 0, 0, 0))
                   + chunk(b'IDAT', zlib.compress(b'\0' + b'\xff\xff' * 3)) + chunk(b'IEND', b''))
        path = self.root / '16bit.png'
        path.write_bytes(encoded)
        with self.assertRaisesRegex(ValueError, '8-bit'):
            comparator.load_image(path)

    def test_pixel_bound_rejects_oversize_images(self):
        old = comparator.MAX_PIXELS
        comparator.MAX_PIXELS = 11
        try:
            with self.assertRaisesRegex(ValueError, 'pixel'):
                comparator.load_image(self.save('reference.png'))
        finally:
            comparator.MAX_PIXELS = old
        self.assertEqual(comparator.load_image(self.root / 'reference.png')['width'], 4)

    def test_color_metadata_mismatch_rejected_without_color_conversion(self):
        info = PngImagePlugin.PngInfo()
        info.add(b'gAMA', struct.pack('>I', 45455))
        a = comparator.load_image(self.save('reference.png'))
        b = comparator.load_image(self.save('candidate.png', pnginfo=info))
        self.assertEqual(a['rgba'], b['rgba'])
        with self.assertRaisesRegex(ValueError, 'color metadata'):
            comparator.compare(a, b, self.provenance(a, b))

    def test_matching_icc_profiles_pass_while_differing_profiles_rejected(self):
        a = comparator.load_image(self.save('reference.png', icc_profile=b'profile-one'))
        b = comparator.load_image(self.save('candidate.png', icc_profile=b'profile-one'))
        self.assertIsNotNone(a['color_metadata']['icc_profile_sha256'])
        self.assertTrue(comparator.compare(a, b, self.provenance(a, b))['exact_equal'])
        other = comparator.load_image(self.save('other.png', icc_profile=b'profile-two'))
        with self.assertRaisesRegex(ValueError, 'color metadata'):
            comparator.compare(a, other, self.provenance(a, other))

    def test_color_key_transparency_is_not_silently_expanded(self):
        path = self.save('key.png', mode='RGB', transparency=(255, 255, 255))
        with self.assertRaisesRegex(ValueError, 'Color-key transparency'):
            comparator.load_image(path)

    def test_orientation_is_not_automatically_changed(self):
        exif = Image.Exif()
        exif[274] = 6
        path = self.save('rotated.png', exif=exif)
        with self.assertRaisesRegex(ValueError, 'orientation'):
            comparator.load_image(path)

    def test_animation_is_not_reduced_to_one_frame(self):
        path = self.root / 'animated.png'
        Image.new('RGBA', (4, 3), 'white').save(path, save_all=True,
            append_images=[Image.new('RGBA', (4, 3), 'black')], duration=100, loop=0)
        with self.assertRaisesRegex(ValueError, 'multiframe'):
            comparator.load_image(path)

    def test_diagnostic_grid_bound_is_explicit_error(self):
        a = comparator.load_image(self.save('reference.png', size=(400, 400)))
        with self.assertRaisesRegex(ValueError, '100000 tiles'):
            comparator.compare(a, a, self.provenance(a, a), region_size=1)

    def test_region_arguments_must_be_positive_integers(self):
        for kwarg in ('region_size', 'max_regions'):
            for value in (0, -1, 1.5, '64', True, None):
                with self.subTest(argument=kwarg, value=value):
                    with self.assertRaisesRegex(ValueError, 'positive integers'):
                        comparator.compare(*self.pair(), **{kwarg: value})

    def test_cli_exit_codes_and_provenance_file_hash(self):
        a, b, provenance = self.pair()
        path = self.root / 'pair.json'
        raw = json.dumps(provenance).encode()
        path.write_bytes(raw)
        argv = ['--reference', str(self.root / 'reference.png'), '--candidate', str(self.root / 'candidate.png'), '--provenance', str(path)]
        with redirect_stdout(io.StringIO()) as output:
            self.assertEqual(comparator.main(argv), 0)
        self.assertEqual(json.loads(output.getvalue())['provenance_file_sha256'], comparator.digest(raw))
        self.save('candidate.png', pixels={(1, 0): (0, 0, 0, 0)})
        with redirect_stdout(io.StringIO()) as output:
            self.assertEqual(comparator.main(argv), 2)  # Old hash cannot bless changed input.
        self.assertFalse(json.loads(output.getvalue())['exact_equal'])
        b = comparator.load_image(self.root / 'candidate.png')
        path.write_text(json.dumps(self.provenance(a, b)))
        with redirect_stdout(io.StringIO()) as output:
            self.assertEqual(comparator.main(argv), 1)
        self.assertEqual(json.loads(output.getvalue())['status'], 'fail')

    def test_cli_reports_rejected_inputs_as_error_envelope(self):
        a, b, provenance = self.pair()
        prov_path = self.root / 'pair.json'
        prov_path.write_text(json.dumps(provenance))
        bad_json = self.root / 'bad.json'
        bad_json.write_bytes(b'not json{')
        invalid = self.root / 'invalid.json'
        invalid.write_text(json.dumps(dict(provenance, schema_version=0)))
        ref = ['--reference', str(self.root / 'reference.png')]
        cand = ['--candidate', str(self.root / 'candidate.png')]
        prov = ['--provenance', str(prov_path)]
        cases = (
            ('missing input file', ['--reference', str(self.root / 'absent.png')] + cand + prov),
            ('malformed provenance JSON', ref + cand + ['--provenance', str(bad_json)]),
            ('invalid provenance', ref + cand + ['--provenance', str(invalid)]),
        )
        for label, argv in cases:
            with self.subTest(case=label):
                with redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(comparator.main(argv), 2)
                result = json.loads(output.getvalue())
                self.assertEqual(result['schema_version'], 1)
                self.assertEqual(result['status'], 'error')
                self.assertFalse(result['exact_equal'])
                self.assertTrue(isinstance(result['error'], str) and result['error'].strip())

    def test_inputs_are_not_mutated(self):
        a, b, provenance = self.pair()
        original = copy.deepcopy(provenance)
        comparator.compare(a, b, provenance)
        self.assertEqual(provenance, original)


if __name__ == '__main__':
    unittest.main()
