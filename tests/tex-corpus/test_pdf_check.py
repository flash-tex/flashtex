import copy
import importlib.util
from pathlib import Path
import unittest

SPEC=importlib.util.spec_from_file_location('pdf_checker',Path(__file__).with_name('pdf_check.py'))
checker=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(checker)


def artifact(width=612,x=72,declared_count=1,stream_length=None):
    stream=f'0 g\nBT\n/F1 12 Tf\n{x} 708 Td\n(Hello) Tj\nET\n'.encode()
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>',
             f'<< /Type /Pages /Kids [3 0 R] /Count {declared_count} >>'.encode(),
             f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} 792] /Contents 4 0 R >>'.encode(),
             f'<< /Length {len(stream) if stream_length is None else stream_length} >>\nstream\n'.encode()+stream+b'\nendstream']
    data=b'%PDF-1.4\n';offsets=[]
    for i,obj in enumerate(objects,1):
        offsets.append(len(data));data+=f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n'
    xref=len(data)
    data+=b'xref\n0 5\n0000000000 65535 f \n'
    for offset in offsets:data+=f'{offset:010} 00000 n \n'.encode()
    data+=f'trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode()
    return data


class PdfChecks(unittest.TestCase):
    def setUp(self):
        self.reply={'payload':{'pages':[{'number':1,'width_pt':612,'height_pt':792,'items':[
            {'kind':'text','text':'Hello','x_pt':72,'baseline_y_pt':84,'font_size_pt':12,'source':None}]}]}}

    def test_generated_structure_placement_and_limitations(self):
        result=checker.inspect(artifact(),self.reply)
        self.assertEqual(result['status'],'pass');self.assertEqual(result['page_count'],1)
        self.assertIn('font embedding and glyph fidelity',result['unverified'])

    def test_corrupt_xref_rejected(self):
        data=artifact().replace(b'0000000009 00000 n',b'0000000010 00000 n')
        with self.assertRaisesRegex(ValueError,'xref offset'):checker.inspect(data,self.reply)

    def test_missing_header_and_eof_rejected(self):
        for data in (artifact()[1:],artifact()[:-7]):
            with self.assertRaises(ValueError):checker.inspect(data,self.reply)

    def test_count_and_box_mismatch_rejected(self):
        for data in (artifact(declared_count=2),artifact(width=600)):
            with self.assertRaises(ValueError):checker.inspect(data,self.reply)

    def test_stream_length_rejected(self):
        with self.assertRaisesRegex(ValueError,'stream length'):checker.inspect(artifact(stream_length=1),self.reply)

    def test_baseline_outside_page_rejected(self):
        self.reply['payload']['pages'][0]['items'][0]['x_pt']=700
        with self.assertRaisesRegex(ValueError,'outside page'):checker.inspect(artifact(x=700),self.reply)

    def test_placement_shift_rejected(self):
        with self.assertRaisesRegex(ValueError,'placement differs'):checker.inspect(artifact(x=73),self.reply)


if __name__=='__main__':unittest.main()
