import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

SPEC=importlib.util.spec_from_file_location('bridge_e2e',Path(__file__).with_name('bridge_e2e.py'))
bridge=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(bridge)


class BridgeEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.evidence={'results':[
            {'scenario':'included-file-context','status':'fail'},
            {'scenario':'macro-context-changed','status':'unverified','context_revision':1,'prepared':{'expected_revision':2}},
            {'scenario':'multibyte-rebase','status':'pass','prepared':{'start_byte':11,'end_byte':11,'expected_revision':2,
                'document_before_sha256':hashlib.sha256('東京αβ world'.encode()).hexdigest()},'source':'東京αβ $x$world'},
            {'scenario':'confirm-restart-idempotency','status':'unverified','bridge_receipt_retry':'pass'}]}
        self.request={'protocol_version':1,'id':'compile','type':'compile','payload':{'project_id':'corpus','revision':3,
                     'entry_path':'main.tex','documents':[{'path':'main.tex','text':'東京x'}]}}
        self.reply={'protocol_version':1,'id':'compile','type':'compile_result','payload':{'project_id':'corpus','revision':3,
                    'status':'ok','diagnostics':[],'pages':[{'items':[{'source':{'path':'main.tex','start_byte':6,'end_byte':7}}]}]}}

    def test_explicit_unverified_native_gates_preserved(self):
        self.assertEqual(len(bridge.validate_harness(self.evidence)),4)

    def test_character_count_rebase_rejected(self):
        self.evidence['results'][2]['prepared']['start_byte']=7
        with self.assertRaisesRegex(ValueError,'shifted'):bridge.validate_harness(self.evidence)

    def test_native_crash_success_cannot_be_inferred(self):
        self.evidence['results'][3]['status']='pass'
        with self.assertRaisesRegex(ValueError,'native crash'):bridge.validate_harness(self.evidence)

    def test_revision_matched_compile_and_synthetic_stale_gate(self):
        process=subprocess.CompletedProcess(['fake'],0,json.dumps(self.reply)+'\n','')
        with patch.object(bridge.subprocess,'run',return_value=process):
            result=bridge.compile_snapshot(self.request,['fake'])
        self.assertTrue(result['source_ranges_valid']);self.assertTrue(result['stale_response_rejected_by_harness'])
        self.assertIn('unverified',result['native_stale_preview_gate'])

    def test_stale_actual_compiler_response_rejected(self):
        self.reply['payload']['revision']=2
        process=subprocess.CompletedProcess(['fake'],0,json.dumps(self.reply)+'\n','')
        with patch.object(bridge.subprocess,'run',return_value=process):
            with self.assertRaisesRegex(ValueError,'stale'):bridge.compile_snapshot(self.request,['fake'])

    def test_split_utf8_source_rejected(self):
        self.reply['payload']['pages'][0]['items'][0]['source']['start_byte']=1
        process=subprocess.CompletedProcess(['fake'],0,json.dumps(self.reply)+'\n','')
        with patch.object(bridge.subprocess,'run',return_value=process):
            self.assertEqual(bridge.compile_snapshot(self.request,['fake'])['transport_status'],'fail')


if __name__=='__main__':unittest.main()
