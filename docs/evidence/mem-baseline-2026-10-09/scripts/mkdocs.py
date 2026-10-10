import os
import shutil
import subprocess
import sys

sys.path.insert(0, '/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a2fa2131a670119f7/tools/incr-bench')
import gen  # noqa: E402

B = '/private/tmp/mb/src'
FX = '/Users/jay3332/Projects/flashtex/fixtures/real-world'


def w(name, text, main='main.tex'):
    os.makedirs(f'{B}/{name}', exist_ok=True)
    open(f'{B}/{name}/{main}', 'w').write(text)


w('blank', "\\documentclass{article}\n\\begin{document}\nHello world, this is a short line of prose to type into here.\n\\end{document}\n")
w('art1', gen.doc(1, False))
w('art4', gen.doc(4, False))
w('full-100', gen.doc(100, True))
os.makedirs(f'{B}/hw1', exist_ok=True)
shutil.copy(f'{FX}/hw1/HW1.tex', f'{B}/hw1/HW1.tex')
os.makedirs(f'{B}/beamer-default', exist_ok=True)
shutil.copy(f'{FX}/beamer-default/main.tex', f'{B}/beamer-default/main.tex')
if not os.path.exists(f'{B}/infdesc'):
    subprocess.run(['rsync', '-a', '--exclude', 'infdesc.pdf', os.path.expanduser('~/Documents/infdesc/'),
                    f'{B}/infdesc/'], check=True)
print(sorted(os.listdir(B)))
