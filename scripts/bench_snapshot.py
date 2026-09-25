#!/usr/bin/env python3
"""Append current target/bench/*.json as a snapshot to benches/history/history.json.

Usage: python3 scripts/bench_snapshot.py [--commit SHA] [--ts ISO]
Defaults: git HEAD short SHA, current UTC time.
"""
import argparse, datetime, json, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HIST = ROOT / "benches" / "history" / "history.json"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--commit", default=None)
    ap.add_argument("--ts", default=None)
    a = ap.parse_args()
    commit = a.commit or subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, cwd=ROOT
    ).stdout.strip()
    ts = a.ts or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    results = {}
    for f in sorted((ROOT / "target" / "bench").glob("*.json")):
        for r in json.loads(f.read_text())["results"]:
            row = {k: r[k] for k in ("p50_ns", "p95_ns", "mean_ns") if k in r}
            if "throughput_bps" in r:
                row["throughput_bps"] = r["throughput_bps"]
            results[r["name"]] = row
    if not results:
        raise SystemExit("no JSONs in target/bench/; run cargo bench first")
    hist = json.loads(HIST.read_text()) if HIST.exists() else []
    hist.append({"ts": ts, "commit": commit, "results": results})
    HIST.write_text(json.dumps(hist, indent=1) + "\n")
    print(f"snapshot {commit} @ {ts}: {len(results)} kpis, {len(hist)} total")


if __name__ == "__main__":
    main()
