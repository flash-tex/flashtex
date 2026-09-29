#!/bin/bash
# Phase 5: bodies a/b with the task's preamble minus siunitx (hyperref default),
# because siunitx inflates hyperref's per-page \pdfstringdef cost ~6x.
set -u
cd "$(dirname "$0")"
export DRIVER=mainh-nosi
python3 bench2.py e 9 empty-pdf-full,empty-dvi-full,plain-pdf-full,plain-discard-full,plain-dvi-full
python3 bench2.py a 9 plain-pdf-full,split1-pdf-full,split1-discard-full,gonly1-full,split1-dvi-full
python3 bench2.py b 9 plain-pdf-full,split1-pdf-full,split1-discard-full,gonly1-full,split1-dvi-full
echo PHASE5-DONE
