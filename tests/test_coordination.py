"""Synthetic Git fixtures never leave temporary directories or invoke a model."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

MODULE = Path(__file__).resolve().parents[1] / 'scripts' / 'coord.py'
spec = importlib.util.spec_from_file_location('coord', MODULE)
coord = importlib.util.module_from_spec(spec)
spec.loader.exec_module(coord)


class CoordinationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.origin = self.base / 'origin.git'
        self.a, self.b = self.base / 'a', self.base / 'b'
        self.cmd(None, 'init', '--bare', '--initial-branch=main', str(self.origin))
        self.cmd(None, 'clone', str(self.origin), str(self.a))
        self.configure(self.a)
        (self.a / 'README').write_text('fixture\n')
        self.commit(self.a, 'fixture baseline')
        self.cmd(self.a, 'push', 'origin', 'HEAD:main')
        self.cmd(None, 'clone', str(self.origin), str(self.b))
        self.configure(self.b)

    def cmd(self, root, *args):
        return coord.git(root, *args)

    def configure(self, root):
        self.cmd(root, 'config', 'user.name', 'Cursor')
        self.cmd(root, 'config', 'user.email', 'cursor@flashtex.invalid')
        self.cmd(root, 'config', 'commit.gpgsign', 'false')

    def commit(self, root, message='synthetic fixture'):
        self.cmd(root, 'add', '.')
        self.cmd(root, 'commit', '-m', message)
        return self.cmd(root, 'rev-parse', 'HEAD')

    def register(self, root=None, agent='worker'):
        root = root or self.a
        self.cmd(root, 'switch', '-c', f'agent/{agent}/register')
        coord.register(root, SimpleNamespace(id=agent, machine='fixture', tool='test-double', capability=['rust'], allocation='fixture'))
        self.commit(root)
        self.cmd(root, 'push', '-u', 'origin', f'agent/{agent}/register')

    def test_legacy_branch_requires_exact_published_assignment(self):
        self.cmd(self.a, 'switch', '-c', 'agent/legacy/product')
        folder = self.a / 'coordination' / 'assignments'
        folder.mkdir(parents=True)
        assignment = {'schema_version': 1, 'agent_id': 'worker',
                      'branch': 'agent/legacy/product', 'state': 'assigned'}
        (folder / 'TASK.json').write_text(json.dumps(assignment))
        with self.assertRaises(ValueError):
            coord.branch(self.a, 'worker')  # unpublished local claim cannot authorize
        self.commit(self.a)
        self.cmd(self.a, 'push', 'origin', 'HEAD:main')
        self.assertEqual(coord.branch(self.a, 'worker'), 'agent/legacy/product')
        with self.assertRaises(ValueError):
            coord.branch(self.a, 'another-worker')
        self.cmd(self.a, 'switch', '-c', 'agent/legacy/other-product')
        with self.assertRaises(ValueError):
            coord.branch(self.a, 'worker')
        self.cmd(self.a, 'switch', 'agent/legacy/product')
        assignment['state'] = 'cancelled'
        (folder / 'TASK.json').write_text(json.dumps(assignment))
        self.commit(self.a)
        self.cmd(self.a, 'push', 'origin', 'HEAD:main')
        with self.assertRaises(ValueError):
            coord.branch(self.a, 'worker')

    def test_discovery_does_not_acknowledge_and_does_not_touch_work(self):
        self.register()
        (self.b / 'README').write_text('local unfinished work\n')
        before = self.cmd(self.b, 'status', '--porcelain')
        report = coord.checkpoint(self.b, emit=False)
        self.assertEqual(len(report['reports']), 1)
        self.assertEqual(report['reports'][0]['report']['reviewed_peers'], {})
        self.assertEqual(before, self.cmd(self.b, 'status', '--porcelain'))
        again = coord.checkpoint(self.b, emit=False)
        self.assertEqual(again['changed_refs'], [])
        self.assertEqual(again['reports'][0]['report']['reviewed_peers'], {})

    def test_assignment_requires_exact_published_revision_ack(self):
        self.register()
        self.cmd(self.b, 'switch', '-c', 'agent/commander/dispatch')
        args = SimpleNamespace(task='FT-TEST', agent='worker', branch='agent/worker/register',
                               objective='fixture', acceptance=['test'], path=['crates/compiler'],
                               depends=[], minutes=10, allocation='fixture')
        coord.dispatch(self.b, args)
        self.commit(self.b)
        self.cmd(self.b, 'push', 'origin', 'HEAD:main')
        first = coord.checkpoint(self.a, emit=False)
        self.assertFalse(first['assignments'][0]['acknowledged'])
        coord.acknowledge(self.a, SimpleNamespace(id='worker', task='FT-TEST', revision=1, adaptation='Accepted fixture task'))
        self.commit(self.a); self.cmd(self.a, 'push')
        self.assertTrue(coord.checkpoint(self.b, emit=False)['assignments'][0]['acknowledged'])
        coord.dispatch(self.b, args)
        self.commit(self.b); self.cmd(self.b, 'push', 'origin', 'HEAD:main')
        second = coord.checkpoint(self.a, emit=False)
        self.assertFalse(second['assignments'][0]['acknowledged'])
        with self.assertRaises(ValueError):
            coord.acknowledge(self.a, SimpleNamespace(id='worker', task='FT-TEST', revision=1, adaptation='stale'))

    def test_inherited_report_ignored_and_malformed_report_isolated(self):
        self.register()
        self.cmd(self.a, 'switch', '-c', 'agent/worker/other')
        self.cmd(self.a, 'push', '-u', 'origin', 'agent/worker/other')
        self.cmd(self.a, 'switch', '-c', 'agent/broken/register')
        path = self.a / 'coordination/agents/broken.json'
        path.write_text('not JSON')
        self.commit(self.a); self.cmd(self.a, 'push', '-u', 'origin', 'agent/broken/register')
        out = coord.checkpoint(self.b, emit=False)
        self.assertEqual(len(out['reports']), 1)
        self.assertTrue(any('invalid report' in w for w in out['warnings']))

    def test_duplicate_reports_warn_and_staleness_is_not_cancellation(self):
        self.register()
        self.cmd(self.a, 'switch', '-c', 'agent/worker/other')
        path = self.a / 'coordination/agents/worker.json'
        obj = json.loads(path.read_text())
        obj.update(branch='agent/worker/other', updated_utc='2026-01-01T00:00:00Z')
        coord.write_json(path, obj)
        self.commit(self.a); self.cmd(self.a, 'push', '-u', 'origin', 'agent/worker/other')
        out = coord.checkpoint(self.b, emit=False)
        self.assertEqual(len(out['reports']), 2)
        self.assertTrue(any('multiple active' in w for w in out['warnings']))
        stale = next(r for r in out['reports'] if r['stale'])
        self.assertEqual(stale['report']['state'], 'registered')

    def test_failed_fetch_retains_snapshot_and_marks_it_stale(self):
        self.register()
        prior = coord.checkpoint(self.b, emit=False)
        self.cmd(self.b, 'remote', 'set-url', 'origin', str(self.base / 'missing.git'))
        with self.assertRaises(RuntimeError):
            coord.checkpoint(self.b, emit=False)
        saved = json.loads((coord.local_state(self.b) / 'checkpoint.json').read_text())
        self.assertFalse(saved['fetch_ok'])
        self.assertEqual(saved['observed_tips'], prior['observed_tips'])

    def test_worktree_local_state_and_expired_watcher(self):
        worktree = self.base / 'worktree'
        self.cmd(self.b, 'worktree', 'add', '-b', 'agent/test/worktree', str(worktree))
        self.assertTrue((worktree / '.git').is_file())
        coord.checkpoint(worktree, emit=False)
        self.assertTrue((coord.local_state(worktree) / 'checkpoint.json').exists())
        with patch.object(coord, 'DEADLINE', '2000-01-01T00:00:00Z'), patch.object(coord, 'checkpoint') as check:
            coord.watch(worktree, SimpleNamespace(interval=60, until='2000-01-01T00:00:00Z'))
            check.assert_not_called()

    def test_dispatch_rejects_overlap_but_not_similar_prefix(self):
        self.cmd(self.b, 'switch', '-c', 'agent/commander/dispatch')
        args = SimpleNamespace(task='ONE', agent='worker', branch='agent/worker/register',
                               objective='fixture', acceptance=['test'], path=['crates/compiler'],
                               depends=[], minutes=10, allocation='fixture')
        coord.dispatch(self.b, args)
        args.task = 'TWO'; args.path = ['crates/compiler/src']
        with self.assertRaises(ValueError): coord.dispatch(self.b, args)
        args.path = ['crates/compiler2']
        coord.dispatch(self.b, args)

    def test_legacy_registration_is_discoverable_without_fake_ack(self):
        self.cmd(self.a, 'switch', '-c', 'agent/legacy/register')
        path = self.a / 'coordination/legacy.md'
        path.parent.mkdir()
        path.write_text('# Legacy report\nAvailable on Mac\n')
        self.commit(self.a); self.cmd(self.a, 'push', '-u', 'origin', 'agent/legacy/register')
        out = coord.checkpoint(self.b, emit=False)
        self.assertEqual(out['legacy_handoffs'][0]['agent_id'], 'legacy')
        self.assertEqual(out['reports'], [])

    def prepare_publish(self):
        self.cmd(self.a, 'switch', '-c', 'agent/test/publish')
        (self.a / 'change').write_text('expected tree\n')
        self.cmd(self.a, 'add', 'change')
        return SimpleNamespace(allocation='fixture', message='fixture publish', implementation='Test double', timeout=10)

    def fake_cursor(self, mutate=False):
        original = coord.run
        def fake(argv, cwd=None, timeout=45, check=True):
            if argv[:3] == ['gh', 'api', 'user']:
                return subprocess.CompletedProcess(argv, 0, '{"login":"fixture-user","id":123}', '')
            if argv[0] != 'cursor-agent':
                return original(argv, cwd=cwd, timeout=timeout, check=check)
            if mutate:
                (Path(cwd) / 'change').write_text('unexpected mutation')
                original(['git', 'add', 'change'], cwd=cwd)
            return original(['git', 'commit', '-m', 'fixture\n\nImplementation-Agent: Test double\nCommit-Executor: Cursor CLI\nCo-authored-by: fixture-user <123+fixture-user@users.noreply.github.com>'], cwd=cwd)
        return fake

    def test_explicit_direct_fallback_never_calls_cursor_and_preserves_coauthor(self):
        args = self.prepare_publish()
        args.direct_agent_commit = True
        trailer = 'Co-authored-by: fixture <123+fixture@users.noreply.github.com>'
        with patch.object(coord, 'current_git_user_trailer', return_value=trailer), patch.object(coord, 'run', wraps=coord.run) as run:
            coord.publish(self.a, args)
        self.assertFalse(any(call.args[0][0] == 'cursor-agent' for call in run.call_args_list))
        message = self.cmd(self.a, 'show', '-s', '--format=%B', 'HEAD')
        self.assertIn('Commit-Executor: git via current agent (authorized Cursor-limit fallback)', message)
        self.assertIn(trailer, message)
        self.assertNotIn('Commit-Executor: Cursor CLI', message)
        self.assertEqual(self.cmd(self.a, 'show', '-s', '--format=%an', 'HEAD'), 'Test double')
        self.assertEqual(self.cmd(self.a, 'rev-parse', 'HEAD'), self.cmd(self.origin, 'rev-parse', 'refs/heads/agent/test/publish'))

    def test_publish_requires_actual_subprocess_then_verifies_exact_tree(self):
        args = self.prepare_publish()
        with patch.object(coord, 'run', side_effect=self.fake_cursor()) as invocation:
            coord.publish(self.a, args)
            self.assertTrue(any(c.args[0][0] == 'cursor-agent' for c in invocation.call_args_list))
        self.assertEqual(self.cmd(self.a, 'rev-parse', 'HEAD'), self.cmd(self.origin, 'rev-parse', 'refs/heads/agent/test/publish'))

    def test_publish_refuses_mutation_and_never_pushes_it(self):
        args = self.prepare_publish()
        with patch.object(coord, 'run', side_effect=self.fake_cursor(mutate=True)):
            with self.assertRaises(RuntimeError): coord.publish(self.a, args)
        self.assertEqual(self.cmd(self.origin, 'for-each-ref', '--format=%(refname)', 'refs/heads/agent/'), '')

    def test_publish_refuses_dirty_state_before_model_call(self):
        args = self.prepare_publish()
        (self.a / 'untracked').write_text('must preserve')
        with patch.object(coord, 'run', wraps=coord.run) as invocation:
            with self.assertRaises(ValueError): coord.publish(self.a, args)
            self.assertFalse(any(c.args[0][0] == 'cursor-agent' for c in invocation.call_args_list))

    def test_non_fast_forward_publication_never_overwrites_peer(self):
        args = self.prepare_publish()
        self.cmd(self.b, 'switch', '-c', 'agent/test/publish')
        (self.b / 'peer').write_text('peer work')
        peer = self.commit(self.b)
        self.cmd(self.b, 'push', '-u', 'origin', 'agent/test/publish')
        with patch.object(coord, 'run', side_effect=self.fake_cursor()):
            with self.assertRaises(RuntimeError): coord.publish(self.a, args)
        self.assertEqual(self.cmd(self.origin, 'rev-parse', 'refs/heads/agent/test/publish'), peer)


if __name__ == '__main__':
    unittest.main()
