use super::*;
#[derive(Serialize)]
struct Record {
    mode: Mode,
    workload: String,
    payload_bytes: usize,
    run: usize,
    calls: usize,
    elapsed_ms: f64,
    throughput_per_s: f64,
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
    db_attempts: u64,
    actual_commits: u64,
    cache_hits: u64,
    joins: u64,
    cpu_ticks: u64,
    rss_kib_before: u64,
    rss_kib_after: u64,
}
fn proc_sample() -> (u64, u64) {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .unwrap()
        .1
        .split_whitespace()
        .collect();
    let ticks = fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap();
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let rss = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (ticks, rss)
}
pub(super) async fn benchmark() {
    let runs = std::env::var("ROM_BENCH_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let calls = std::env::var("ROM_BENCH_CALLS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400usize);
    for run in 0..runs {
        for size in [64, 4096, 65536] {
            for workload in ["unique-cold", "hot-warm", "mixed-warm", "burst-cold"] {
                for order in 0..4 {
                    let mode = Mode::ALL[(order + run) % 4];
                    let dir = tempfile::tempdir().unwrap();
                    let path = dir.path().join("PROTOTYPE-bench.sqlite");
                    let r = Runtime::new(&path, mode, 128, 4096, 3_600_000);
                    let warm = matches!(workload, "hot-warm" | "mixed-warm");
                    if warm {
                        for id in 0..16 {
                            assert!(r.call(key(id), input(size)).await.is_ok());
                        }
                    }
                    let s = &r.shared.stats;
                    let before = [
                        s.attempts.load(Ordering::Relaxed),
                        s.commits.load(Ordering::Relaxed),
                        s.hits.load(Ordering::Relaxed),
                        s.joins.load(Ordering::Relaxed),
                    ];
                    let (cpu, rss) = proc_sample();
                    let start = Instant::now();
                    let mut latencies = Vec::with_capacity(calls);
                    if workload == "burst-cold" {
                        for batch in (0..calls).step_by(32) {
                            let mut jobs = Vec::new();
                            let barrier =
                                Arc::new(tokio::sync::Barrier::new((calls - batch).min(32)));
                            for _ in batch..(batch + 32).min(calls) {
                                let r = r.clone();
                                let barrier = barrier.clone();
                                let i = input(size);
                                jobs.push(tokio::spawn(async move {
                                    barrier.wait().await;
                                    let t = Instant::now();
                                    let result = r.call(key(batch / 32), i).await;
                                    assert!(result.is_ok());
                                    t.elapsed().as_secs_f64() * 1e6
                                }));
                            }
                            for j in jobs {
                                latencies.push(j.await.unwrap());
                            }
                        }
                    } else {
                        for n in 0..calls {
                            let id = match workload {
                                "hot-warm" => n % 16,
                                "mixed-warm" => {
                                    if n % 5 == 0 {
                                        16 + n
                                    } else {
                                        n % 16
                                    }
                                }
                                _ => n,
                            };
                            let i = input(size);
                            let t = Instant::now();
                            assert!(r.call(key(id), i).await.is_ok());
                            latencies.push(t.elapsed().as_secs_f64() * 1e6);
                        }
                    }
                    let elapsed = start.elapsed().as_secs_f64();
                    let (cpu_after, rss_after) = proc_sample();
                    latencies.sort_by(f64::total_cmp);
                    let percentile =
                        |p: f64| latencies[((calls as f64 * p).ceil() as usize).saturating_sub(1)];
                    let expected = match workload {
                        "hot-warm" => 16,
                        "mixed-warm" => 16 + calls.div_ceil(5),
                        "burst-cold" => calls.div_ceil(32),
                        _ => calls,
                    };
                    assert_eq!(
                        counts(&path),
                        (expected as i64, expected as i64, expected as i64)
                    );
                    println!(
                        "{}",
                        serde_json::to_string(&Record {
                            mode,
                            workload: workload.into(),
                            payload_bytes: size,
                            run,
                            calls,
                            elapsed_ms: elapsed * 1000.,
                            throughput_per_s: calls as f64 / elapsed,
                            p50_us: percentile(0.50),
                            p95_us: percentile(0.95),
                            p99_us: percentile(0.99),
                            db_attempts: s.attempts.load(Ordering::Relaxed) - before[0],
                            actual_commits: s.commits.load(Ordering::Relaxed) - before[1],
                            cache_hits: s.hits.load(Ordering::Relaxed) - before[2],
                            joins: s.joins.load(Ordering::Relaxed) - before[3],
                            cpu_ticks: cpu_after - cpu,
                            rss_kib_before: rss,
                            rss_kib_after: rss_after
                        })
                        .unwrap()
                    );
                }
            }
        }
    }
}
