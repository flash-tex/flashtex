import re, subprocess
new = open('/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-ab46fa803477079b1/crates/flashtex-engine/src/arena.rs').read()
old = open('/tmp/mfp/arena-old.rs').read()
# shaped() and restore_cost from the new file
shaped = new[new.index('    /// A word whose halves take every packed width'):new.index('    #[test]\n    fn packed_words_round_trip')]
rc = new[new.index('    /// The cost of a restore through many sealed logs'):]
rc = rc[:rc.index('    #[test]\n    fn parallel_restore_is_exact')]
old = old.replace('            arr[i] = x;\n        }\n    }\n', '            arr[i] = shaped(x);\n        }\n    }\n\n' + shaped, 1)
old = old.replace('    #[test]\n    fn parallel_restore_is_exact', rc + '    #[test]\n    fn parallel_restore_is_exact', 1)
open('/tmp/mfp/arena-oldbench.rs', 'w').write(old)
