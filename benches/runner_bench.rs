//! Runner `plan()` latency per language, stdlib only.
//! Batch 100 plan() calls per sample so each sample (~10us)
//! dwarfs timer overhead (~20-40ns); report per-plan ns.
//!
//! Run: `cargo bench --bench runner_bench`
//! Output: `target/bench/runner.json`

use std::fs;
use std::time::Instant;

use upmd_runner::{find, CodeInput, StateCaptureContext};

fn percentile(sorted_ns: &mut [u128], pct: f64) -> u128 {
    sorted_ns.sort_unstable();
    if sorted_ns.is_empty() {
        return 0;
    }
    let idx = ((pct / 100.0) * (sorted_ns.len() as f64)).floor() as usize;
    sorted_ns[idx.min(sorted_ns.len() - 1)]
}

fn main() {
    // plan() builds ExecutionPlan only; no binary lookup, no spawn.
    // Covers inline path (single line) + file path (multiline).
    let cases: &[(&str, &str)] = &[
        ("bash", "echo hello"),
        ("sh", "echo hello"),
        ("python", "print('hello')"),
        ("js", "console.log('hi')"),
        ("go", "package main\nfunc main() {}"),
        ("rust", "fn main() {}"),
    ];
    let no_capture = StateCaptureContext {
        enabled: false,
        fifos: None,
        code_id: 0,
    };
    let mut rows = Vec::new();
    for (lang, content) in cases {
        let runner = match find(lang) {
            Ok(r) => r,
            Err(_) => continue, // skip unknown alias; registry decides
        };
        let language = runner.language().clone();
        let iters = 2000usize;
        let batch = 100usize;
        for _ in 0..10 {
            for _ in 0..batch {
                let input = CodeInput {
                    id: 1,
                    content,
                    language: &language,
                    state_capture: &no_capture,
                };
                let _ = std::hint::black_box(runner.plan(&input));
            }
        }
        let mut samples = Vec::with_capacity(iters);
        for _ in 0..iters {
            let start = Instant::now();
            for _ in 0..batch {
                let input = CodeInput {
                    id: 1,
                    content,
                    language: &language,
                    state_capture: &no_capture,
                };
                let plan = runner.plan(&input).expect("plan succeeds");
                std::hint::black_box(plan);
            }
            samples.push(start.elapsed().as_nanos() / batch as u128);
        }
        let mean_ns = samples.iter().sum::<u128>() / samples.len() as u128;
        let p50 = percentile(&mut samples.clone(), 50.0);
        let p95 = percentile(&mut samples, 95.0);
        rows.push(format!(
            "    {{\"name\": \"plan_{lang}\", \"iters\": {iters}, \"batch\": {batch}, \"mean_ns\": {mean_ns}, \"p50_ns\": {p50}, \"p95_ns\": {p95}}}"
        ));
    }
    let json = format!(
        "{{\n  \"bench\": \"runner\",\n  \"results\": [\n{}\n  ]\n}}\n",
        rows.join(",\n")
    );
    fs::create_dir_all("target/bench").expect("create target/bench");
    fs::write("target/bench/runner.json", &json).expect("write runner.json");
    println!("{json}");
}
