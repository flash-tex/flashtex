"""Experimental display-list contract tests; synthetic GIDs are NOT font fixtures.

Run: /tmp/flashtex-schema-venv/bin/python -m unittest discover -s tests
-p test_rendering_v2.py (install jsonschema==4.23.0 in an isolated venv).
"""
import copy
import hashlib
import importlib.util
import math
from pathlib import Path
import struct
import unittest

SPEC = importlib.util.spec_from_file_location('rendering', Path(__file__).parents[1]/'scripts/check_rendering_v2.py')
rendering = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(rendering)
FEATURES = ['glyph_run','rule','static-truetype','rgba-srgb','cluster-actualtext']

def dl2_header_digest(pl):
    """Appendix A `header_digest` (docs/proposals/display-list-v2-delta.md)."""
    def i64(n):
        return struct.pack('<q', int(n))
    def s(t):
        b=t.encode('utf-8'); return i64(len(b))+b
    def ranges(rs):
        out=i64(len(rs))
        for r in rs: out += s(r['path'])+i64(r['start_byte'])+i64(r['end_byte'])
        return out
    h=hashlib.sha256(b'flashtex:dl2:header:1\0')
    for k in ('render_format','coordinate_unit','color_space','text_extraction','project_id'): h.update(s(pl[k]))
    h.update(i64(pl['revision']))
    h.update(i64(len(pl['required_features'])))
    for f in pl['required_features']: h.update(s(f))
    h.update(i64(len(pl['documents'])))
    for d in pl['documents']: h.update(s(d['path'])+i64(d['revision'])+s(d['sha256'])+i64(d['byte_length']))
    h.update(i64(len(pl['fonts'])))
    for f in pl['fonts']:
        h.update(s(f['font_id'])+s(f['sha256'])+i64(f['byte_length'])+s(f['format'])+i64(f['face_index'])+i64(f['units_per_em'])+i64(f['glyph_count'])+s(f['postscript_name']))
    h.update(i64(len(pl['diagnostics'])))
    for d in pl['diagnostics']:
        h.update(s(d['code'])+s(d['message'])+s(d['severity'])+ranges(d['sources']))
        sug=d.get('suggestion')
        if sug: h.update(s(sug))
    return h.hexdigest()

def sample():
    text='office e\u0301'
    data=text.encode()
    offer={'protocol_version':2,'id':'hello','type':'render_capabilities','payload':{'render_formats':['display-list-v2'],'features':FEATURES[:]}}
    source={'path':'main.tex','start_byte':0,'end_byte':len(data)}
    cluster={'text_start_byte':0,'text_end_byte':len(data),'sources':[source], 'hit_rects':[{'x':0,'top':0,'width':100,'height':100}], 'carets':[]}
    glyph={'gid':1,'origin_x':0,'baseline_y':90,'advance_x':100,'advance_y':0,'cluster':0}
    run={'kind':'glyph_run','font_id':'synthetic','font_size':12582912,'text':text,'glyphs':[glyph,dict(glyph,gid=2,origin_x=100,advance_x=0)],'clusters':[cluster],'paint':{'r':0,'g':0,'b':0,'a':1}}
    doc={'path':'main.tex','revision':1,'sha256':hashlib.sha256(data).hexdigest(),'byte_length':len(data)}
    font={'font_id':'synthetic','sha256':'a'*64,'byte_length':128,'format':'static-truetype','face_index':0,'units_per_em':1000,'glyph_count':3,'postscript_name':'SyntheticOnly'}
    message={'protocol_version':2,'id':'compile-2','type':'display_list','payload':{'render_format':'display-list-v2','coordinate_unit':'bp_2pow20','color_space':'srgb','text_extraction':'cluster-actualtext','project_id':'p','revision':2,'required_features':FEATURES[:],'documents':[doc],'fonts':[font],'pages':[{'number':1,'width':1000000,'height':1000000,'items':[run]}],'diagnostics':[]}}
    return offer,message,{'main.tex':{'revision':1,'text':text}}


class RenderingTests(unittest.TestCase):
    def setUp(self):
        self.offer,self.message,self.documents=sample()
        self.item=self.message['payload']['pages'][0]['items'][0]

    def validate(self):
        return rendering.validate(self.message,offer=self.offer,documents=self.documents)

    def invalid(self):
        with self.assertRaises(rendering.InvalidRendering):self.validate()

    def test_schema_is_valid_draft202012(self):
        import json
        rendering.Draft202012Validator.check_schema(json.loads(rendering.SCHEMA.read_text()))

    def test_cluster_mapping_preserves_logical_text_once(self):
        before=copy.deepcopy(self.message)
        result=self.validate()
        self.assertTrue(result['source_snapshots_verified'])
        self.assertFalse(result['font_bytes_verified'])
        self.assertFalse(result['paintable'])
        clusters=self.item['clusters'];raw=self.item['text'].encode()
        self.assertEqual(b''.join(raw[c['text_start_byte']:c['text_end_byte']] for c in clusters),raw)
        self.assertEqual(self.message,before)

    def test_no_implicit_v2_negotiation(self):
        with self.assertRaises(rendering.InvalidRendering):rendering.validate(self.message)
        self.offer['payload']['render_formats']=['runtime-v1'];self.invalid()

    def test_selection_must_correlate_offer(self):
        selection={'protocol_version':2,'id':'hello','type':'render_format_selected','payload':{'render_format':'display-list-v2','required_features':FEATURES[:]}}
        self.assertEqual(rendering.validate(selection,offer=self.offer)['status'],'selected')
        selection['id']='wrong'
        with self.assertRaises(rendering.InvalidRendering):rendering.validate(selection,offer=self.offer)

    def test_unknown_message_and_primitive_are_rejected(self):
        original=copy.deepcopy(self.message)
        self.message['type']='compile_result';self.invalid()
        self.message=original;self.message['payload']['pages'][0]['items'][0]['kind']='image';self.invalid()

    def test_missing_capability_is_not_silent_skip(self):
        self.offer['payload']['features'].remove('glyph_run');self.invalid()

    def test_undeclared_primitive_fails(self):
        self.message['payload']['required_features'].remove('glyph_run');self.invalid()

    def test_nonfinite_numbers_rejected(self):
        for value in (math.nan,math.inf,-math.inf):
            with self.subTest(value=value):
                self.item['paint']['a']=value;self.invalid()

    def test_integer_tick_limit_and_sum_overflow(self):
        glyph=self.item['glyphs'][0]
        glyph['origin_x']=2**53;self.invalid()
        glyph['origin_x']=2**53-1;glyph['advance_x']=1;self.invalid()

    def test_unknown_font_and_out_of_range_gid(self):
        self.item['font_id']='missing';self.invalid()
        self.item['font_id']='synthetic';self.item['glyphs'][0]['gid']=3;self.invalid()

    def test_notdef_is_explicitly_rejected(self):
        self.item['glyphs'][0]['gid']=0;self.invalid()

    def test_unknown_cluster_and_unmapped_cluster(self):
        self.item['glyphs'][0]['cluster']=7;self.invalid()

    def test_cluster_cannot_split_utf8_or_omit_text(self):
        self.item['clusters'][0]['text_end_byte']-=1;self.invalid()

    def test_source_boundaries_are_checked_against_exact_snapshot(self):
        self.item['clusters'][0]['sources'][0]['end_byte']-=1;self.invalid()

    def test_source_snapshot_revision_and_digest_are_pinned(self):
        self.documents['main.tex']['revision']=2;self.invalid()
        self.documents['main.tex']['revision']=1;self.documents['main.tex']['text']='different';self.invalid()

    def test_duplicate_font_and_document_ids_rejected(self):
        self.message['payload']['fonts']*=2;self.invalid()
        self.message['payload']['fonts']=self.message['payload']['fonts'][:1]
        self.message['payload']['documents']*=2;self.invalid()

    def test_generated_content_requires_explicit_provenance(self):
        cluster=self.item['clusters'][0]
        del cluster['sources'];self.invalid()
        cluster['synthetic_reason']='Generated equation number';self.validate()

    def test_font_bytes_cannot_be_substituted(self):
        with self.assertRaisesRegex(rendering.InvalidRendering,'Font digest'):
            rendering.validate(self.message,offer=self.offer,documents=self.documents,font_bytes={'synthetic':b'wrong'})

    def test_rule_is_typed_geometry_not_box_drawing_text(self):
        self.message['payload']['pages'][0]['items'].append({'kind':'rule','x':0,'top':10,'width':100,'height':2,'paint':{'r':0,'g':0,'b':0,'a':1},'synthetic_reason':'Fraction bar'})
        self.validate()
        self.message['payload']['pages'][0]['items'][-1]['height']=0;self.invalid()

    def test_unsafe_source_paths_rejected(self):
        for path in ('../secret','/tmp/file','a/../b','a\\b','a//b','a/./b'):
            with self.subTest(path=path):
                self.message['payload']['documents'][0]['path']=path;self.invalid()

    def test_invalid_unicode_and_excessive_nesting_rejected(self):
        self.item['text']='\ud800';self.invalid()
        value={}
        for _ in range(66):value={'nested':value}
        with self.assertRaises(rendering.InvalidRendering):rendering.validate(value)

    def test_font_header_metadata_checked_without_claiming_outline_validity(self):
        import struct
        # Deliberately synthetic sfnt header only, not a licensed/renderable font.
        head=bytearray(54);struct.pack_into('>H',head,18,1000)
        maxp=bytearray(6);struct.pack_into('>H',maxp,4,3)
        tables={b'head':bytes(head),b'maxp':bytes(maxp),b'glyf':b'',b'loca':b''}
        offset=12+len(tables)*16
        directory=b'';contents=b''
        for tag,body in tables.items():
            directory+=struct.pack('>4sIII',tag,0,offset,len(body))
            contents+=body;offset+=len(body)
        data=struct.pack('>IHHHH',0x10000,len(tables),0,0,0)+directory+contents
        font=self.message['payload']['fonts'][0]
        font.update(sha256=hashlib.sha256(data).hexdigest(),byte_length=len(data))
        result=rendering.validate(self.message,offer=self.offer,documents=self.documents,font_bytes={'synthetic':data})
        self.assertTrue(result['font_bytes_verified'])
        self.assertTrue(result['resources_ready_for_renderer_validation'])
        self.assertFalse(result['paintable'])
        font['glyph_count']=4
        with self.assertRaisesRegex(rendering.InvalidRendering,'glyph count'):
            rendering.validate(self.message,offer=self.offer,documents=self.documents,font_bytes={'synthetic':data})

    def test_rejection_envelope_never_paints(self):
        rejected={'protocol_version':2,'id':'hello','type':'render_format_rejected','payload':{'code':'unsupported_feature','message':'image is unsupported'}}
        self.assertEqual(rendering.validate(rejected),{'status':'rejected','paintable':False})

    def test_header_digest_includes_suggestion_when_present(self):
        # Appendix A walk-through of a diagnostics-enabled payload whose only
        # extra field is suggestion "\\alpha" (same vector as delta.rs).
        payload={'render_format':'display-list-v2','coordinate_unit':'bp_2pow20','color_space':'srgb','text_extraction':'cluster-actualtext','project_id':'p','revision':1,'required_features':['glyph_run','rgba-srgb','cluster-actualtext'],'documents':[],'fonts':[],'diagnostics':[{'code':'unknown_command','message':'\\alpah','severity':'error','sources':[{'path':'notes.tex','start_byte':0,'end_byte':6}],'suggestion':'\\alpha'}]}
        self.assertEqual(dl2_header_digest(payload),'c4e7c7129994d1b73c8dfe3d9b1b9a0cbf0edc49e7f9e6bc848d8c66e0126bb6')
        off=copy.deepcopy(payload); del off['diagnostics'][0]['suggestion']
        self.assertEqual(dl2_header_digest(off),'e554935e8987d50810a82c72274b661d6be747d2b41993b0d008715cad5f8dbe')
        empty=copy.deepcopy(payload); empty['diagnostics'][0]['suggestion']=''
        self.assertEqual(dl2_header_digest(empty),dl2_header_digest(off))


if __name__=='__main__':unittest.main()
