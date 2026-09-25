//! Parse benchmarks, stdlib only (`std::time::Instant`, no criterion).
//!
use std::fs;
use std::time::Instant;

fn percentile(sorted_ns: &mut [u128], pct: f64) -> u128 {
    sorted_ns.sort_unstable();
    if sorted_ns.is_empty() {
        return 0;
    }
    let idx = ((pct / 100.0) * (sorted_ns.len() as f64)).floor() as usize;
    sorted_ns[idx.min(sorted_ns.len() - 1)]
}

struct Stats {
    iters: usize,
    mean_ns: u128,
    p50_ns: u128,
    p95_ns: u128,
    bytes: usize,
}

fn measure(iters: usize, warmup: usize, bytes: usize, mut f: impl FnMut() -> u128) -> Stats {
    for _ in 0..warmup {
        f();
    }
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        samples.push(f());
    }
    let mean_ns = samples.iter().sum::<u128>() / samples.len() as u128;
    let p50_ns = percentile(&mut samples.clone(), 50.0);
    let p95_ns = percentile(&mut samples, 95.0);
    Stats {
        iters,
        mean_ns,
        p50_ns,
        p95_ns,
        bytes,
    }
}

fn throughput_bps(bytes: usize, mean_ns: u128) -> u64 {
    if mean_ns == 0 {
        return 0;
    }
    (bytes as u128 * 1_000_000_000 / mean_ns) as u64
}

fn small_md() -> String {
    "# Demo\n\nSome text with **bold** and `code`.\n\n```bash\necho hello\n```\n\n## Tasks\n\n- [ ] one\n- [x] two\n".to_string()
}

fn large_md() -> String {
    let mut s = String::from("---\ntitle: bench\n---\n\n# Large doc\n\n");
    for i in 0..2000 {
        s.push_str(&format!(
            "## Section {i}\n\nParagraph {i} with *emphasis* and a [link](https://example.com/{i}).\n\n```bash\necho block {i}\n```\n\n- [ ] task {i}\n\n| a | b |\n|---|---|\n| {i} | {i} |\n\n"
        ));
    }
    s
}

fn bench_case(name: &str, source: &str, iters: usize) -> String {
    let parser = upmd_parser::Parser::new();
    // Clone outside timed region: measures parse only, not allocator.
    let stats = measure(iters, 3, source.len(), || {
        let input = source.to_string();
        let start = Instant::now();
        let doc = parser.parse(input);
        let elapsed = start.elapsed().as_nanos();
        std::hint::black_box(doc.codes.len() + doc.headings.len());
        elapsed
    });
    format!(
        "    {{\"name\": \"{name}\", \"iters\": {}, \"bytes\": {}, \"mean_ns\": {}, \"p50_ns\": {}, \"p95_ns\": {}, \"throughput_bps\": {}}}",
        stats.iters,
        stats.bytes,
        stats.mean_ns,
        stats.p50_ns,
        stats.p95_ns,
        throughput_bps(stats.bytes, stats.mean_ns)
    )
}

fn main() {
    let small = small_md();
    let large = large_md();
    // parse_large at 50 iters keeps `cargo bench` fast (~1s) with stable p95.
    let rows = [
        bench_case("parse_small", &small, 500),
        bench_case("parse_large", &large, 50),
    ];
    let json = format!(
        "{{\n  \"bench\": \"parse\",\n  \"results\": [\n{}\n  ]\n}}\n",
        rows.join(",\n")
    );
    fs::create_dir_all("target/bench").expect("create target/bench");
    fs::write("target/bench/parse.json", &json).expect("write parse.json");
    println!("{json}");
}
