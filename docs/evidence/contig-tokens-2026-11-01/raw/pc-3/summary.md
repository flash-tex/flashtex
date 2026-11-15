## exact instructions (cachegrind Ir), delta against main
| document | main | lazy |
|---|---:|---:|
| 2501.06980v1 | - | - |
| 2501.07014v3 | - | - |
| 2501.07044v1 | - | - |
| 2501.07084v3 | - | - |
| 2501.07124v3 | - | - |
| 2501.07198v2 | - | - |
| 2501.07301v2 | - | - |
| 2501.07372v2 | - | - |
| 2501.07497v1 | - | - |
| 2501.07623v3 | - | - |
| 2501.07840v2 | - | - |
| 2501.08157v1 | - | - |
| 2501.09225v1 | - | - |
| infdesc-x2 | - | - |

## native runs: median user s (perf instructions / cycles when present), per part

### pc
| document | main | lazy |
|---|---:|---:|
| 2501.06980v1 | 0.52 (6.85 G / 2.40 G) | 0.48 (6.79 G / 2.26 G) -7.3 % |
| 2501.07014v3 | 0.88 (10.39 G / 4.14 G) | 0.86 (10.36 G / 4.06 G) -2.0 % |
| 2501.07044v1 | 0.77 (9.16 G / 3.62 G) | 0.75 (9.12 G / 3.53 G) -2.6 % |
| 2501.07084v3 | 0.41 (5.60 G / 1.88 G) | 0.40 (5.54 G / 1.81 G) -3.6 % |
| 2501.07124v3 | 5.07 (55.32 G / 24.04 G) | 5.02 (55.18 G / 23.73 G) -0.9 % |
| 2501.07198v2 | 0.55 (6.95 G / 2.58 G) | 0.54 (6.91 G / 2.53 G) -1.8 % |
| 2501.07301v2 | 0.72 (10.09 G / 3.38 G) | 0.69 (9.98 G / 3.25 G) -3.9 % |
| 2501.07372v2 | 0.33 (4.30 G / 1.48 G) | 0.31 (4.25 G / 1.43 G) -4.0 % |
| 2501.07497v1 | 1.18 (17.39 G / 5.65 G) | 1.09 (17.15 G / 5.24 G) -7.6 % |
| 2501.07623v3 | 0.30 (3.77 G / 1.37 G) | 0.29 (3.74 G / 1.31 G) -3.0 % |
| 2501.07840v2 | 0.41 (5.11 G / 1.88 G) | 0.40 (5.08 G / 1.89 G) -1.0 % |
| 2501.08157v1 | 0.88 (12.38 G / 4.13 G) | 0.84 (12.26 G / 3.96 G) -4.5 % |
| 2501.09225v1 | 0.60 (8.42 G / 2.77 G) | 0.58 (8.35 G / 2.73 G) -2.2 % |
| infdesc-x2 | 36.76 (450.25 G / 175.17 G) | 34.52 (447.50 G / 164.08 G) -6.1 % |

## output identity against main
DIFFERS: 2501.07084v3 lazy native rep 0 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 lazy native rep 1 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 lazy native rep 2 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 lazy native rep 3 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 lazy native rep 4 part pc: pdf True, log False, mem True
140 runs compared, 5 differ from main's first run of the same document
exit codes: {('main', 0): 72, ('lazy', 0): 72, ('main', 1): 12, ('lazy', 1): 12}
