"""Synthetic Git fixtures never leave temporary directories or invoke a model."""
from datetime import datetime, timedelta, timezone
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / 'scripts' / 'coord.py'
spec = importlib.util.spec_from_file_location('coord', MODULE)
coord = importlib.util.module_from_spec(spec)
spec.loader.exec_module(coord)


class ClaimsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.origin = self.base / 'origin.git'
        self.cmd(None, 'init', '--bare', '-q', '--initial-branch=main', str(self.origin))
        self.a = self.clone('a')
        self.b = self.clone('b')

    def cmd(self, root, *args):
        return subprocess.run(['git', *args], cwd=root, text=True, capture_output=True, check=True).stdout.strip()

    def clone(self, name, actor=None):
        root = self.base / name
        if not root.exists():
            self.cmd(None, 'clone', '-q', str(self.origin), str(root))
        self.cmd(root, 'config', 'user.name', f'Test {name}')
        self.cmd(root, 'config', 'user.email', f'{name}@test.invalid')
        self.cmd(root, 'config', 'commit.gpgsign', 'false')
        return root

    def seed_main(self):
        (self.a / 'README').write_text('fixture\n')
        self.cmd(self.a, 'add', '-A')
        self.cmd(self.a, 'commit', '-q', '-m', 'base')
        self.cmd(self.a, 'push', '-q', 'origin', 'HEAD:main')
        self.cmd(self.b, 'fetch', '-q', 'origin')
        self.cmd(self.b, 'checkout', '-q', 'main')

    def claims_json(self, root, task):
        return json.loads(self.cmd(root, 'show', f'origin/{coord.CLAIMS_BRANCH}:claims/{task}.json'))

    # -- creation race: two independent clones race to create the orphan branch

    def test_create_branch_race_first_push_wins_loser_retries(self):
        self.seed_main()
        real_run = coord.run
        started = {'a': False}

        def wrapper(argv, cwd=None, timeout=45, check=True):
            if (not started['a'] and len(argv) >= 4 and argv[:3] == ['git', 'push', 'origin']
                    and argv[3].endswith(f':refs/heads/{coord.CLAIMS_BRANCH}')):
                started['a'] = True
                # b races in and creates the branch first, for an unrelated task.
                rc = coord.claim_cmd(self.b, SimpleNamespace(
                    task='FT-OTHER', actor='worker-b', machine='mac-b', branch=None, gh_ref=None, note=None))
                self.assertEqual(rc, 0)
            return real_run(argv, cwd=cwd, timeout=timeout, check=check)

        with patch.object(coord, 'run', side_effect=wrapper):
            rc = coord.claim_cmd(self.a, SimpleNamespace(
                task='FT-RACE', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        self.assertEqual(rc, 0)
        self.assertEqual(self.claims_json(self.a, 'FT-RACE')['actor'], 'worker-a')
        self.assertEqual(self.claims_json(self.a, 'FT-OTHER')['actor'], 'worker-b')

    def test_retry_on_push_rejection_between_fetch_and_push(self):
        self.seed_main()
        # b creates the branch up front so the race below is an ordinary
        # fast-forward rejection, not the orphan-creation special case.
        self.assertEqual(coord.claim_cmd(self.b, SimpleNamespace(
            task='FT-SEED', actor='worker-b', machine='mac-b', branch=None, gh_ref=None, note=None)), 0)

        real_run = coord.run
        fired = {'once': False}

        def wrapper(argv, cwd=None, timeout=45, check=True):
            if (not fired['once'] and len(argv) >= 4 and argv[:3] == ['git', 'push', 'origin']
                    and argv[3].endswith(f':refs/heads/{coord.CLAIMS_BRANCH}')):
                fired['once'] = True
                rc = coord.claim_cmd(self.b, SimpleNamespace(
                    task='FT-INTERLEAVED', actor='worker-b', machine='mac-b', branch=None, gh_ref=None, note=None))
                self.assertEqual(rc, 0)
            return real_run(argv, cwd=cwd, timeout=timeout, check=check)

        with patch.object(coord, 'run', side_effect=wrapper):
            rc = coord.claim_cmd(self.a, SimpleNamespace(
                task='FT-RACE2', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        self.assertEqual(rc, 0)
        self.assertTrue(fired['once'])
        self.assertEqual(self.claims_json(self.a, 'FT-RACE2')['actor'], 'worker-a')
        self.assertEqual(self.claims_json(self.a, 'FT-INTERLEAVED')['actor'], 'worker-b')

    # -- claim / LOST

    def test_claim_lost_when_another_actor_holds_it(self):
        self.seed_main()
        self.assertEqual(coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-1', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None)), 0)
        rc = coord.claim_cmd(self.b, SimpleNamespace(
            task='FT-1', actor='worker-b', machine='mac-b', branch=None, gh_ref=None, note=None))
        self.assertEqual(rc, 1)
        self.assertEqual(self.claims_json(self.b, 'FT-1')['actor'], 'worker-a')

    # -- idempotent refresh

    def test_claim_is_idempotent_for_same_actor(self):
        self.seed_main()
        coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-2', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note='n1'))
        first = self.claims_json(self.a, 'FT-2')
        with patch.object(coord, 'stamp', return_value='2099-01-01T00:00:00Z'):
            rc = coord.claim_cmd(self.a, SimpleNamespace(
                task='FT-2', actor='worker-a', machine='mac-a', branch='agent/worker-a/ft2', gh_ref=None, note=None))
        self.assertEqual(rc, 0)
        second = self.claims_json(self.a, 'FT-2')
        self.assertEqual(second['started_utc'], first['started_utc'])
        self.assertEqual(second['updated_utc'], '2099-01-01T00:00:00Z')
        self.assertEqual(second['branch'], 'agent/worker-a/ft2')
        self.assertEqual(second['note'], 'n1')  # preserved, not overwritten with None

    # -- release / close authorization

    def test_release_requires_holder_or_matching_commander_force(self):
        self.seed_main()
        coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-3', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        with self.assertRaises(ValueError):
            coord.release_cmd(self.b, SimpleNamespace(task='FT-3', actor='worker-b', force_by_commander=None, note=None))
        (self.b / 'coordination').mkdir(exist_ok=True)
        (self.b / 'coordination' / 'authority.json').write_text(json.dumps(
            {'schema_version': 1, 'commander_id': 'the-commander'}))
        with self.assertRaises(ValueError):
            coord.release_cmd(self.b, SimpleNamespace(
                task='FT-3', actor='worker-b', force_by_commander='someone-else', note=None))
        rc = coord.release_cmd(self.b, SimpleNamespace(
            task='FT-3', actor='worker-b', force_by_commander='the-commander', note='forced'))
        self.assertEqual(rc, 0)
        self.assertEqual(self.claims_json(self.b, 'FT-3')['state'], 'released')

    def test_close_by_holder_sets_gh_ref(self):
        self.seed_main()
        coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-4', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        rc = coord.close_cmd(self.a, SimpleNamespace(
            task='FT-4', actor='worker-a', force_by_commander=None, gh_ref='https://example.invalid/pr/1', note=None))
        self.assertEqual(rc, 0)
        closed = self.claims_json(self.a, 'FT-4')
        self.assertEqual(closed['state'], 'closed')
        self.assertEqual(closed['gh_ref'], 'https://example.invalid/pr/1')
        with self.assertRaises(ValueError):
            coord.close_cmd(self.a, SimpleNamespace(
                task='FT-4', actor='worker-a', force_by_commander=None, gh_ref=None, note=None))

    # -- stale detection with fake timestamps

    def test_stale_detection_by_age_and_branch_activity(self):
        self.seed_main()
        old = (datetime.now(timezone.utc) - timedelta(hours=100)).isoformat(timespec='seconds').replace('+00:00', 'Z')
        recent = (datetime.now(timezone.utc) - timedelta(hours=1)).isoformat(timespec='seconds').replace('+00:00', 'Z')

        def write_claim(root, task, obj):
            scratch = self.base / f'stage-{task}'
            if self._claims_branch_exists(root):
                self.cmd(root, 'worktree', 'add', '--detach', str(scratch), f'origin/{coord.CLAIMS_BRANCH}')
            else:
                self._create_orphan_worktree(root, scratch)
            (scratch / 'claims').mkdir(parents=True, exist_ok=True)
            (scratch / 'claims' / f'{task}.json').write_text(json.dumps(obj))
            self.cmd(scratch, 'add', '-A')
            self.cmd(scratch, 'commit', '-q', '-m', f'stage {task}')
            self.cmd(scratch, 'push', '-q', 'origin', f'HEAD:refs/heads/{coord.CLAIMS_BRANCH}')
            self.cmd(root, 'worktree', 'remove', '--force', str(scratch))

        # A claim without a branch: stale purely by age once old enough.
        write_claim(self.a, 'FT-STALE-NOBRANCH', {
            'task_id': 'FT-STALE-NOBRANCH', 'actor': 'worker-a', 'machine': 'mac-a', 'branch': None,
            'gh_ref': None, 'started_utc': old, 'updated_utc': old, 'state': 'claimed', 'note': None})

        # A claim with a branch that has a fresh commit: NOT stale.
        self.cmd(self.a, 'checkout', '-q', '-b', 'agent/worker-a/fresh')
        (self.a / 'work.txt').write_text('recent work\n')
        self.cmd(self.a, 'add', '-A')
        self.cmd(self.a, 'commit', '-q', '-m', 'recent work')
        self.cmd(self.a, 'push', '-q', 'origin', 'HEAD:agent/worker-a/fresh')
        self.cmd(self.a, 'checkout', '-q', 'main')
        write_claim(self.a, 'FT-STALE-FRESHBRANCH', {
            'task_id': 'FT-STALE-FRESHBRANCH', 'actor': 'worker-a', 'machine': 'mac-a',
            'branch': 'agent/worker-a/fresh', 'gh_ref': None, 'started_utc': old, 'updated_utc': old,
            'state': 'claimed', 'note': None})

        # A recent claim: not stale regardless of branch.
        write_claim(self.a, 'FT-FRESH', {
            'task_id': 'FT-FRESH', 'actor': 'worker-a', 'machine': 'mac-a', 'branch': None,
            'gh_ref': None, 'started_utc': recent, 'updated_utc': recent, 'state': 'claimed', 'note': None})

        rc = coord.claims_cmd(self.a, SimpleNamespace(json=True, stale=48, actor=None, touch=None))
        self.assertEqual(rc, 2)
        tip = coord._fetch_claims_tip(self.a)
        all_open = {t: c for t, c in coord._read_all_claims(self.a, f'origin/{coord.CLAIMS_BRANCH}').items()
                    if c['state'] == 'claimed'}
        stale = {t: c for t, c in all_open.items() if coord._is_stale(self.a, c, 48)}
        self.assertEqual(set(stale), {'FT-STALE-NOBRANCH'})

    def _claims_branch_exists(self, root):
        self.cmd(root, 'fetch', '-q', 'origin')
        return bool(subprocess.run(['git', 'rev-parse', '--verify', '--quiet', f'origin/{coord.CLAIMS_BRANCH}'],
                                    cwd=root, text=True, capture_output=True).stdout.strip())

    def _create_orphan_worktree(self, root, scratch):
        self.cmd(root, 'worktree', 'add', '--detach', '--no-checkout', str(scratch), 'HEAD')
        self.cmd(scratch, 'checkout', '--orphan', 'stage-orphan')
        self.cmd(scratch, 'read-tree', '--empty')

    # -- touch / claims listing / checkpoint surfacing

    def test_touch_requires_holder_and_refreshes_updated(self):
        self.seed_main()
        coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-5', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        first = self.claims_json(self.a, 'FT-5')
        with self.assertRaises(ValueError):
            coord.claims_cmd(self.a, SimpleNamespace(json=False, stale=None, actor='someone-else', touch='FT-5'))
        with patch.object(coord, 'stamp', return_value='2099-06-01T00:00:00Z'):
            rc = coord.claims_cmd(self.a, SimpleNamespace(json=False, stale=None, actor='worker-a', touch='FT-5'))
        self.assertEqual(rc, 0)
        second = self.claims_json(self.a, 'FT-5')
        self.assertEqual(second['updated_utc'], '2099-06-01T00:00:00Z')
        self.assertEqual(second['started_utc'], first['started_utc'])

    def test_checkpoint_reports_callers_open_and_stale_claims(self):
        self.seed_main()
        self.cmd(self.a, 'checkout', '-q', '-b', 'agent/worker-a/ft6')
        coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-6', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        report = coord.checkpoint(self.a, emit=False)
        self.assertIn('FT-6', report['claims']['caller_open'])
        self.assertEqual(report['claims']['caller_actor'], 'worker-a')

    # -- the caller's own checkout is never touched

    def test_callers_checkout_is_never_touched(self):
        self.seed_main()
        self.cmd(self.a, 'checkout', '-q', '-b', 'agent/worker-a/untouched')
        (self.a / 'scratch-untracked.txt').write_text('leave me alone\n')
        before_head = self.cmd(self.a, 'rev-parse', 'HEAD')
        before_branch = self.cmd(self.a, 'symbolic-ref', '--short', 'HEAD')
        before_status = self.cmd(self.a, 'status', '--porcelain')

        coord.claim_cmd(self.a, SimpleNamespace(
            task='FT-7', actor='worker-a', machine='mac-a', branch=None, gh_ref=None, note=None))
        coord.claims_cmd(self.a, SimpleNamespace(json=False, stale=None, actor='worker-a', touch='FT-7'))
        coord.release_cmd(self.a, SimpleNamespace(task='FT-7', actor='worker-a', force_by_commander=None, note=None))

        self.assertEqual(self.cmd(self.a, 'rev-parse', 'HEAD'), before_head)
        self.assertEqual(self.cmd(self.a, 'symbolic-ref', '--short', 'HEAD'), before_branch)
        self.assertEqual(self.cmd(self.a, 'status', '--porcelain'), before_status)
        worktrees = self.cmd(self.a, 'worktree', 'list', '--porcelain')
        self.assertEqual(worktrees.count('worktree '), 1)


if __name__ == '__main__':
    unittest.main()
