#!/usr/bin/env python3
"""MEASUREMENT ONLY (lane TYPING-50MS): how late a thread sleeping 50 ms wakes on this machine,
the typist's question (dl3-keys --interval-ms). Prints p50/p95/max of the overshoot in ms."""
import sys
import time

n = int(sys.argv[1]) if len(sys.argv) > 1 else 40
late = []
t0 = time.monotonic()
for k in range(1, n + 1):
    due = t0 + 0.05 * k
    d = due - time.monotonic()
    if d > 0:
        time.sleep(d)
    late.append((time.monotonic() - due) * 1e3)
late.sort()
print(f"{sys.argv[2] if len(sys.argv) > 2 else 'probe'}: sleep-50ms overshoot p50 {late[len(late) // 2]:.1f} "
      f"p95 {late[int(0.95 * (len(late) - 1))]:.1f} max {late[-1]:.1f} ms")
