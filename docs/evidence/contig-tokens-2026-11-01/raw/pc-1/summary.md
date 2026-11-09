## exact instructions (cachegrind Ir), delta against main
| document | main | off | on |
|---|---:|---:|---:|
| 2501.06980v1 | - | - | - |
| 2501.07014v3 | - | - | - |
| 2501.07044v1 | - | - | - |
| 2501.07084v3 | - | - | - |
| 2501.07124v3 | - | - | - |
| 2501.07198v2 | - | - | - |
| 2501.07301v2 | - | - | - |
| 2501.07372v2 | - | - | - |
| 2501.07497v1 | - | - | - |
| 2501.07623v3 | - | - | - |
| 2501.07840v2 | - | - | - |
| 2501.08157v1 | - | - | - |
| 2501.09225v1 | - | - | - |
| infdesc-x2 | - | - | - |

## native runs: median user s (perf instructions / cycles when present), per part

### pc
| document | main | off | on |
|---|---:|---:|---:|
| 2501.06980v1 | 0.55 (6.85 G / 2.56 G) | 0.54 (6.94 G / 2.50 G) -2.0 % | 0.53 (6.82 G / 2.47 G) -2.9 % |
| 2501.07014v3 | 0.92 (10.39 G / 4.30 G) | 0.91 (10.45 G / 4.28 G) -1.3 % | 0.91 (10.38 G / 4.24 G) -1.2 % |
| 2501.07044v1 | 0.74 (9.16 G / 3.52 G) | 0.74 (9.24 G / 3.52 G) -0.1 % | 0.73 (9.14 G / 3.45 G) -1.9 % |
| 2501.07084v3 | 0.41 (5.60 G / 1.92 G) | 0.41 (5.69 G / 1.93 G) -0.5 % | 0.40 (5.57 G / 1.82 G) -4.6 % |
| 2501.07124v3 | 5.11 (55.32 G / 24.33 G) | 5.11 (55.57 G / 24.36 G) +0.1 % | 5.04 (55.26 G / 24.13 G) -1.2 % |
| 2501.07198v2 | 0.55 (6.95 G / 2.60 G) | 0.55 (7.02 G / 2.58 G) +0.7 % | 0.54 (6.93 G / 2.53 G) -2.0 % |
| 2501.07301v2 | 0.71 (10.09 G / 3.35 G) | 0.72 (10.27 G / 3.38 G) +1.6 % | 0.67 (10.03 G / 3.13 G) -5.4 % |
| 2501.07372v2 | 0.32 (4.30 G / 1.48 G) | 0.33 (4.36 G / 1.53 G) +2.8 % | 0.30 (4.27 G / 1.42 G) -5.0 % |
| 2501.07497v1 | 1.30 (17.39 G / 6.05 G) | 1.30 (17.77 G / 6.05 G) +0.4 % | 1.19 (17.27 G / 5.53 G) -8.4 % |
| 2501.07623v3 | 0.29 (3.77 G / 1.35 G) | 0.29 (3.82 G / 1.36 G) +1.4 % | 0.30 (3.75 G / 1.35 G) +2.1 % |
| 2501.07840v2 | 0.42 (5.11 G / 1.98 G) | 0.44 (5.20 G / 2.04 G) +3.3 % | 0.41 (5.10 G / 1.88 G) -3.8 % |
| 2501.08157v1 | 0.91 (12.38 G / 4.25 G) | 0.88 (12.60 G / 4.18 G) -2.3 % | 0.85 (12.33 G / 3.99 G) -6.1 % |
| 2501.09225v1 | 0.58 (8.42 G / 2.75 G) | 0.59 (8.56 G / 2.79 G) +1.5 % | 0.56 (8.40 G / 2.66 G) -4.6 % |
| infdesc-x2 | 37.00 (450.25 G / 175.70 G) | 36.59 (458.68 G / 173.65 G) -1.1 % | 34.54 (450.40 G / 163.96 G) -6.7 % |

## output identity against main
DIFFERS: 2501.07084v3 off native rep 0 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 on native rep 0 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 off native rep 1 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 on native rep 1 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 on native rep 2 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 off native rep 2 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 off native rep 3 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 on native rep 3 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 off native rep 4 part pc: pdf True, log False, mem True
DIFFERS: 2501.07084v3 on native rep 4 part pc: pdf True, log False, mem True
210 runs compared, 10 differ from main's first run of the same document
exit codes: {('main', 0): 72, ('off', 0): 72, ('on', 0): 72, ('main', 1): 12, ('off', 1): 12, ('on', 1): 12}
