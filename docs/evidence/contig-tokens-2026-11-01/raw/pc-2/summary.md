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
| 2501.06980v1 | 0.49 (6.85 G / 2.32 G) | 0.49 (6.95 G / 2.36 G) +0.2 % | 0.47 (6.81 G / 2.22 G) -4.1 % |
| 2501.07014v3 | 0.86 (10.39 G / 4.08 G) | 0.86 (10.45 G / 4.09 G) -0.2 % | 0.84 (10.37 G / 4.04 G) -1.7 % |
| 2501.07044v1 | 0.74 (9.16 G / 3.51 G) | 0.75 (9.24 G / 3.54 G) +1.2 % | 0.72 (9.14 G / 3.46 G) -2.4 % |
| 2501.07084v3 | 0.40 (5.60 G / 1.86 G) | 0.40 (5.69 G / 1.88 G) +1.0 % | 0.39 (5.57 G / 1.80 G) -2.3 % |
| 2501.07124v3 | 4.93 (55.32 G / 23.79 G) | 4.95 (55.57 G / 23.85 G) +0.5 % | 4.89 (55.23 G / 23.63 G) -0.8 % |
| 2501.07198v2 | 0.56 (6.95 G / 2.61 G) | 0.55 (7.02 G / 2.61 G) -1.3 % | 0.54 (6.93 G / 2.55 G) -3.2 % |
| 2501.07301v2 | 0.70 (10.09 G / 3.34 G) | 0.71 (10.28 G / 3.39 G) +2.0 % | 0.66 (10.02 G / 3.16 G) -5.1 % |
| 2501.07372v2 | 0.31 (4.30 G / 1.45 G) | 0.32 (4.37 G / 1.46 G) +0.6 % | 0.30 (4.27 G / 1.40 G) -5.4 % |
| 2501.07497v1 | 1.18 (17.39 G / 5.62 G) | 1.19 (17.78 G / 5.70 G) +1.3 % | 1.09 (17.25 G / 5.20 G) -6.8 % |
| 2501.07623v3 | 0.29 (3.77 G / 1.35 G) | 0.29 (3.83 G / 1.35 G) +3.1 % | 0.28 (3.75 G / 1.29 G) -2.4 % |
| 2501.07840v2 | 0.39 (5.11 G / 1.86 G) | 0.40 (5.20 G / 1.87 G) +1.0 % | 0.39 (5.10 G / 1.81 G) -1.3 % |
| 2501.08157v1 | 0.90 (12.38 G / 4.25 G) | 0.89 (12.60 G / 4.19 G) -1.4 % | 0.89 (12.30 G / 4.20 G) -1.3 % |
| 2501.09225v1 | 0.61 (8.42 G / 2.89 G) | 0.62 (8.56 G / 2.96 G) +1.0 % | 0.59 (8.38 G / 2.77 G) -4.4 % |
| infdesc-x2 | 35.10 (450.25 G / 169.33 G) | 35.02 (458.07 G / 168.87 G) -0.2 % | 33.78 (448.64 G / 162.16 G) -3.8 % |

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
