#!/usr/bin/env python3
"""Compare current bench JSON vs baseline, fail on >5% regression.

Usage:
    python3 scripts/bench_compare.py --current target/bench --baseline target/bench-baseline
    python3 scripts/bench_compare.py --current target/bench --baseline target/bench-baseline --threshold 5.0

JSON shape (per file): {"bench": "<name>", "results": [{"name": ..., "p50_ns": ..., "p95_ns": ..., ...}]}
Compares p50_ns primarily; p95_ns reported as info. Missing baseline file or
missing benchmark name = warning, not failure (pass).
Exit 0 = pass, 1 = regression beyond threshold.
Stdlib only.
"""

import argparse
import json
import os
import sys

METRIC = "p50_ns"


def load_results(path):
    with open(path) as f:
        data = json.load(f)
    out = {}
    for r in data.get("results", []):
        if "name" in r and METRIC in r:
            out[r["name"]] = r
    return out


def main():
    ap = argparse.ArgumentParser(description="Fail on bench regression over threshold.")
    ap.add_argument("--current", required=True, help="Dir with current *.json")
    ap.add_argument("--baseline", required=True, help="Dir with baseline *.json")
    ap.add_argument("--threshold", type=float, default=5.0, help="Allowed regression pct (default 5.0)")
    args = ap.parse_args()

    if not os.path.isdir(args.baseline):
        print(f"WARN: no baseline dir {args.baseline}; skipping comparison (pass).")
        return 0
    current_files = [f for f in os.listdir(args.current) if f.endswith(".json")]
    if not current_files:
        print(f"ERROR: no JSON in {args.current}")
        return 1

    failed = False
    compared = 0
    for fname in sorted(current_files):
        cur_path = os.path.join(args.current, fname)
        base_path = os.path.join(args.baseline, fname)
        if not os.path.exists(base_path):
            print(f"WARN: no baseline for {fname}; skipping.")
            continue
        cur = load_results(cur_path)
        base = load_results(base_path)
        for name, c in sorted(cur.items()):
            if name not in base:
                print(f"WARN: {fname}/{name} not in baseline; skipping.")
                continue
            b = base[name][METRIC]
            v = c[METRIC]
            if b == 0:
                print(f"WARN: {fname}/{name} baseline is 0; skipping.")
                continue
            delta = (v - b) / b * 100.0
            p95info = ""
            if "p95_ns" in c and "p95_ns" in base[name] and base[name]["p95_ns"]:
                p95d = (c["p95_ns"] - base[name]["p95_ns"]) / base[name]["p95_ns"] * 100.0
                p95info = f" (p95 {p95d:+.1f}%)"
            status = "OK " if delta <= args.threshold else "FAIL"
            print(f"[{status}] {fname}/{name}: {b} -> {v} ns ({delta:+.1f}%){p95info}")
            compared += 1
            if delta > args.threshold:
                failed = True

    if compared == 0:
        print("WARN: nothing compared; pass.")
        return 0
    if failed:
        print(f"ERROR: regression over +{args.threshold}% detected.")
        return 1
    print("PASS: no regression over threshold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
