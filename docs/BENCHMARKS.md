# Benchmarks

Stdlib-only harness (`std::time::Instant`). No criterion, no new deps.

## KPIs

- **p50 latency (ns)** — median per-iteration time. Primary regression signal.
- **p95 latency (ns)** — tail latency. Reported alongside p50; not a gate.
- **throughput (bytes/s)** — parse benches only (`bytes / mean_ns`). Higher is better.
- **alloc count** — not measured yet. `ponytail:` ceiling is manual counting via
  `dhat`/`valgrind --tool=massif`; add when parse latency regresses without an
  obvious cause.

## What is measured

| Bench | Cases | Metric |
|---|---|---|
| `benches/parse_bench.rs` | `parse_small` (~200 B doc), `parse_large` (~2000 sections) | p50/p95 + throughput |
| `benches/runner_bench.rs` | `plan_<lang>` for bash, sh, python, js, go, rust | p50/p95 of `plan()` only (no binary lookup, no spawn) |

## Run locally

```sh
cargo bench --bench parse_bench
cargo bench --bench runner_bench
# JSON lands in target/bench/parse.json, target/bench/runner.json
```

Save a baseline before a perf-sensitive change:

```sh
mkdir -p target/bench-baseline
cp target/bench/*.json target/bench-baseline/
# ... make change ...
cargo bench --bench parse_bench && cargo bench --bench runner_bench
python3 scripts/bench_compare.py --current target/bench --baseline target/bench-baseline
```

Custom threshold (default 5%):

```sh
python3 scripts/bench_compare.py --current target/bench --baseline target/bench-baseline --threshold 10
```

## CI

`bench` job in `.github/workflows/ci.yml` (ubuntu, after `test`):

1. Runs both benches.
2. Uploads `target/bench/*.json` as `bench-results` artifact.
3. Copies `benches/baselines/*.json` to `target/bench-baseline/` and runs
   `scripts/bench_compare.py` unconditionally (`continue-on-error: true`).

Variance note: fixed-iteration p50/p95 on shared runners carries ~±10%
natural variance. The 5% gate applies to like-for-like same-machine
comparisons (local baseline vs local current). CI acts as trend warning,
not hard merge blocker. Recalibrate `benches/baselines/` from CI-produced
numbers once available: download the `bench-results` artifact from a main
run and copy into `benches/baselines/`.

Refresh procedure: `cargo bench --bench parse_bench --bench runner_bench`
then `cp target/bench/*.json benches/baselines/`.

## Measured deltas (bbd48b1 → this branch, same machine)

- `parse_large` p50: 7105358 → 5712970 ns (-19.6%)
- `parse_small` p50: 2884 → 2834 ns (-1.7%)
- `plan_python/js/go/rust` p50: -98% to -99% (binary-resolution caching;
  pre-refactor runs spawned `which`/version probes per `plan()`)
- `plan_bash/sh` p50: -8% to -16%

## Read results

JSON shape:

```json
{
  "bench": "parse",
  "results": [
    {"name": "parse_small", "iters": 500, "bytes": 123, "mean_ns": 456,
     "p50_ns": 450, "p95_ns": 600, "throughput_bps": 273333}
  ]
}
```

Compare `p50_ns` current vs baseline. `throughput_bps` moves inversely to
latency — if p50 rises 10%, throughput should fall ~9%.

## Regression triage

1. Re-run locally 2–3×; noisy CI runners cause false positives. Consistent
   >5% across runs = real.
2. `git stash` the change, re-bench, confirm delta disappears.
3. Narrow: `parse_small` slow = per-document overhead (parser construction,
   frontmatter path); `parse_large` slow = per-node cost (block loop,
   allocation per section); `plan_<lang>` slow = that runner's `plan()`.
4. Check for accidental allocation: `String`/`Vec` in hot loop, `.clone()`
   of `Language`, repeated `which` lookup.
5. Fix, re-bench, update baseline JSON if the new number is the intended
   trade-off (note reason in PR).
