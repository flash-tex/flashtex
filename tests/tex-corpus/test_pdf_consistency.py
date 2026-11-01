import importlib.util
from pathlib import Path
import unittest

SPEC=importlib.util.spec_from_file_location('pdf_consistency',Path(__file__).with_name('pdf_consistency.py'))
checker=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(checker)


def sample(pages):
    envelope={'payload':{'pages':[]}}
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>',
             ('<< /Type /Pages /Kids ['+' '.join(f'{3+2*i} 0 R' for i in range(len(pages)))+f'] /Count {len(pages)} >>').encode()]
    for index,texts in enumerate(pages):
        page={'number':index+1,'width_pt':612,'height_pt':792,'items':[]};stream=b'0 g\n'
        for item_index,text in enumerate(texts):
            y=84+item_index*20
            page['items'].append({'kind':'text','text':text,'x_pt':72,'baseline_y_pt':y,'font_size_pt':12,
                                  'source':{'path':f'p{index}.tex','start_byte':0,'end_byte':len(text.encode())}})
            encoded=checker.expected_bytes(text)[0]
            escaped=encoded.replace(b'\\',b'\\\\').replace(b'(',b'\\(').replace(b')',b'\\)')
            stream+=f'BT\n/F1 12 Tf\n72 {792-y} Td\n('.encode()+escaped+b') Tj\nET\n'
        envelope['payload']['pages'].append(page)
        objects.append(f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {4+2*index} 0 R >>'.encode())
        objects.append(f'<< /Length {len(stream)} >>\nstream\n'.encode()+stream+b'\nendstream')
    data=b'%PDF-1.4\n';offsets=[]
    for number,obj in enumerate(objects,1):
        offsets.append(len(data));data+=f'{number} 0 obj\n'.encode()+obj+b'\nendobj\n'
    offset=len(data);size=len(objects)+1
    data+=f'xref\n0 {size}\n0000000000 65535 f \n'.encode()
    for pos in offsets:data+=f'{pos:010} 00000 n \n'.encode()
    data+=f'trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{offset}\n%%EOF\n'.encode()
    return data,envelope


class ConsistencyTests(unittest.TestCase):
    def test_unicode_substitution_requires_specific_warning(self):
        data,reply=sample([['Café 東京']])
        with self.assertRaisesRegex(ValueError,'warning'):checker.inspect(data,reply,'','unsupported')
        warning="warning: page 1: item 0 text: U+6771 U+4EAC not representable in WinAnsiEncoding; written as '?'"
        result=checker.inspect(data,reply,warning,'unsupported')
        self.assertEqual(result['status'],'pass');self.assertFalse(result['text_preserved'])
        self.assertEqual(result['semantic_gate'],'unsupported')

    def test_warning_for_wrong_item_does_not_excuse_loss(self):
        data,reply=sample([['東京']])
        warning="warning: page 1: item 1 text: U+6771 U+4EAC not representable in WinAnsiEncoding; written as '?'"
        with self.assertRaisesRegex(ValueError,'warning'):checker.inspect(data,reply,warning,'fail')

    def test_escaping_and_winansi_literal_bytes(self):
        data,reply=sample([['(Café) \\ € —']])
        result=checker.inspect(data,reply,'','fail')
        self.assertTrue(result['text_preserved']);self.assertEqual(result['compiler_semantic_status'],'fail')

    def test_multiline_and_multipage_provenance(self):
        data,reply=sample([['First','Second'],['Third']])
        result=checker.inspect(data,reply,'','unverified')
        self.assertEqual([(i['page_number'],i['item_index']) for i in result['text_items']],[(1,0),(1,1),(2,0)])
        self.assertEqual(result['text_items'][-1]['source']['path'],'p1.tex')

    def test_same_length_wrong_text_detected(self):
        data,reply=sample([['Hello']]);data=data.replace(b'(Hello)',b'(Wrong)')
        with self.assertRaisesRegex(ValueError,'text bytes'):checker.inspect(data,reply,'','fail')

    def test_dropped_or_duplicated_runtime_items_detected(self):
        for duplicate in (False,True):
            data,reply=sample([['First','Second']])
            if duplicate:reply['payload']['pages'][0]['items'].append(reply['payload']['pages'][0]['items'][1])
            else:reply['payload']['pages'][0]['items'].pop()
            with self.assertRaisesRegex(ValueError,'run count'):checker.inspect(data,reply,'','fail')

    def test_controls_are_substitutions_not_printable_cp1252(self):
        encoded,lost=checker.expected_bytes('a\n\x81😀')
        self.assertEqual(encoded,b'a???');self.assertEqual(lost,['\n','\x81','😀'])

    def test_out_of_bounds_detected(self):
        data,reply=sample([['Hello']]);data=data.replace(b'72 708 Td',b'72 999 Td')
        with self.assertRaisesRegex(ValueError,'outside page'):checker.inspect(data,reply,'','fail')


if __name__=='__main__':unittest.main()
