//! Local performance sampling without user content, identifiers or paths.

use serde::Serialize;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PerformanceSample {
    pub elapsed_ms: u64,
    pub latency_ms: Option<u64>,
    pub process_cpu_percent: Option<f64>,
    pub resident_memory_bytes: Option<u64>,
    pub disk_bytes: u64,
    pub battery_percent: Option<f64>,
    pub temperature_celsius: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PerformancePlatform {
    Linux,
    Windows,
    Android,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PerformanceBudget {
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub resident_memory_bytes: u64,
}

impl PerformancePlatform {
    pub const fn budget(self) -> PerformanceBudget {
        match self {
            Self::Linux | Self::Windows => PerformanceBudget {
                latency_p50_ms: 10_000,
                latency_p95_ms: 30_000,
                resident_memory_bytes: 4 * 1024 * 1024 * 1024,
            },
            Self::Android => PerformanceBudget {
                latency_p50_ms: 20_000,
                latency_p95_ms: 60_000,
                resident_memory_bytes: 3 * 1024 * 1024 * 1024,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PerformanceSummary {
    pub sample_count: usize,
    pub latency_sample_count: usize,
    pub memory_sample_count: usize,
    pub latency_p50_ms: Option<u64>,
    pub latency_p95_ms: Option<u64>,
    pub peak_resident_memory_bytes: Option<u64>,
    pub peak_disk_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum BudgetViolation {
    MissingLatency,
    MissingMemory,
    LatencyP50,
    LatencyP95,
    ResidentMemory,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BudgetVerdict {
    pub passed: bool,
    pub violations: Vec<BudgetViolation>,
}

impl PerformanceSummary {
    pub fn from_samples(samples: &[PerformanceSample]) -> Self {
        let mut latencies: Vec<u64> = samples
            .iter()
            .filter_map(|sample| sample.latency_ms)
            .collect();
        latencies.sort_unstable();
        let memory_sample_count = samples
            .iter()
            .filter(|sample| sample.resident_memory_bytes.is_some())
            .count();
        Self {
            sample_count: samples.len(),
            latency_sample_count: latencies.len(),
            memory_sample_count,
            latency_p50_ms: percentile(&latencies, 50),
            latency_p95_ms: percentile(&latencies, 95),
            peak_resident_memory_bytes: samples
                .iter()
                .filter_map(|sample| sample.resident_memory_bytes)
                .max(),
            peak_disk_bytes: samples
                .iter()
                .map(|sample| sample.disk_bytes)
                .max()
                .unwrap_or(0),
        }
    }

    pub fn evaluate(&self, budget: PerformanceBudget) -> BudgetVerdict {
        let mut violations = Vec::new();
        match self.latency_p50_ms {
            Some(value) if value > budget.latency_p50_ms => {
                violations.push(BudgetViolation::LatencyP50)
            }
            None => violations.push(BudgetViolation::MissingLatency),
            _ => {}
        }
        if self
            .latency_p95_ms
            .is_some_and(|value| value > budget.latency_p95_ms)
        {
            violations.push(BudgetViolation::LatencyP95);
        }
        match self.peak_resident_memory_bytes {
            Some(value) if value > budget.resident_memory_bytes => {
                violations.push(BudgetViolation::ResidentMemory)
            }
            None => violations.push(BudgetViolation::MissingMemory),
            _ => {}
        }
        BudgetVerdict {
            passed: violations.is_empty(),
            violations,
        }
    }
}

fn percentile(sorted_values: &[u64], percentile: usize) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let rank = percentile
        .saturating_mul(sorted_values.len())
        .div_ceil(100)
        .max(1);
    sorted_values.get(rank - 1).copied()
}

#[derive(Debug)]
pub struct PerformanceCollector {
    started: Instant,
    previous_elapsed: Duration,
    previous_cpu: Option<Duration>,
    samples: Vec<PerformanceSample>,
}

impl Default for PerformanceCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceCollector {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            previous_elapsed: Duration::ZERO,
            previous_cpu: process_cpu_time(),
            samples: Vec::new(),
        }
    }

    pub fn sample(
        &mut self,
        monitored_file: &Path,
        latency: Option<Duration>,
    ) -> PerformanceSample {
        let elapsed = self.started.elapsed();
        let cpu = process_cpu_time();
        let process_cpu_percent = cpu.and_then(|current| {
            let previous = self.previous_cpu?;
            cpu_percent(
                current.saturating_sub(previous),
                elapsed.saturating_sub(self.previous_elapsed),
            )
        });
        let sample = PerformanceSample {
            elapsed_ms: millis(elapsed),
            latency_ms: latency.map(millis),
            process_cpu_percent,
            resident_memory_bytes: resident_memory_bytes(),
            disk_bytes: std::fs::metadata(monitored_file)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
            battery_percent: battery_percent(),
            temperature_celsius: temperature_celsius(),
        };
        self.previous_elapsed = elapsed;
        self.previous_cpu = cpu;
        self.samples.push(sample.clone());
        sample
    }

    pub fn samples(&self) -> &[PerformanceSample] {
        &self.samples
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn cpu_percent(cpu_delta: Duration, wall_delta: Duration) -> Option<f64> {
    if wall_delta.is_zero() {
        return None;
    }
    Some(cpu_delta.as_secs_f64() / wall_delta.as_secs_f64() * 100.0)
}

#[cfg(target_os = "linux")]
fn process_cpu_time() -> Option<Duration> {
    let nanoseconds = std::fs::read_to_string("/proc/self/schedstat")
        .ok()?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_nanos(nanoseconds))
}

#[cfg(not(target_os = "linux"))]
fn process_cpu_time() -> Option<Duration> {
    None
}

#[cfg(target_os = "linux")]
fn resident_memory_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    parse_kib_field(&status, "VmRSS:").map(|kib| kib.saturating_mul(1024))
}

#[cfg(not(target_os = "linux"))]
fn resident_memory_bytes() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn battery_percent() -> Option<f64> {
    for entry in std::fs::read_dir("/sys/class/power_supply").ok()? {
        let path = entry.ok()?.path();
        if read_trimmed(path.join("type")).as_deref() == Some("Battery") {
            return read_trimmed(path.join("capacity"))?.parse().ok();
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn battery_percent() -> Option<f64> {
    None
}

#[cfg(target_os = "linux")]
fn temperature_celsius() -> Option<f64> {
    let mut readings = Vec::new();
    for entry in std::fs::read_dir("/sys/class/thermal").ok()? {
        let path = entry.ok()?.path();
        let Some(raw) = read_trimmed(path.join("temp")) else {
            continue;
        };
        if let Ok(milli_celsius) = raw.parse::<f64>() {
            readings.push(milli_celsius / 1000.0);
        }
    }
    readings.into_iter().reduce(f64::max)
}

#[cfg(not(target_os = "linux"))]
fn temperature_celsius() -> Option<f64> {
    None
}

#[cfg(target_os = "linux")]
fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
}

#[cfg(target_os = "linux")]
fn parse_kib_field(input: &str, field: &str) -> Option<u64> {
    input.lines().find_map(|line| {
        let value = line.strip_prefix(field)?.trim();
        value.split_whitespace().next()?.parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_cpu_from_process_and_wall_deltas() {
        assert_eq!(
            cpu_percent(Duration::from_millis(250), Duration::from_secs(1)),
            Some(25.0)
        );
        assert_eq!(cpu_percent(Duration::ZERO, Duration::ZERO), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_resident_memory_without_retaining_the_source() {
        let status = "Name:\tinterviewscribe\nVmRSS:\t  1234 kB\n";
        assert_eq!(parse_kib_field(status, "VmRSS:"), Some(1234));
    }

    #[test]
    fn sample_serialization_contains_only_aggregate_metrics() {
        let sample = PerformanceSample {
            elapsed_ms: 10,
            latency_ms: Some(4),
            process_cpu_percent: Some(12.5),
            resident_memory_bytes: Some(1024),
            disk_bytes: 44,
            battery_percent: None,
            temperature_celsius: None,
        };
        let json = serde_json::to_string(&sample).unwrap();
        assert!(json.contains("\"latency_ms\":4"));
        assert!(!json.contains("audio"));
        assert!(!json.contains("text"));
        assert!(!json.contains("path"));
        assert!(!json.contains("speaker"));
    }

    #[test]
    fn collector_measures_file_growth_without_exposing_its_path() {
        let path = std::env::temp_dir().join(format!(
            "interviewscribe-performance-{}-private-name.wav",
            std::process::id()
        ));
        std::fs::write(&path, [0_u8; 64]).unwrap();
        let mut collector = PerformanceCollector::new();
        let sample = collector.sample(&path, Some(Duration::from_millis(7)));

        assert_eq!(sample.disk_bytes, 64);
        assert_eq!(sample.latency_ms, Some(7));
        assert_eq!(collector.samples(), &[sample]);
        assert!(!serde_json::to_string(collector.samples())
            .unwrap()
            .contains("private-name"));
        std::fs::remove_file(path).ok();
    }

    fn metric_sample(latency_ms: Option<u64>, memory_bytes: Option<u64>) -> PerformanceSample {
        PerformanceSample {
            elapsed_ms: 0,
            latency_ms,
            process_cpu_percent: None,
            resident_memory_bytes: memory_bytes,
            disk_bytes: 64,
            battery_percent: None,
            temperature_celsius: None,
        }
    }

    #[test]
    fn computes_nearest_rank_p50_p95_and_peaks() {
        let samples: Vec<_> = (1..=20)
            .map(|value| metric_sample(Some(value * 100), Some(value * 1024)))
            .collect();
        let summary = PerformanceSummary::from_samples(&samples);

        assert_eq!(summary.sample_count, 20);
        assert_eq!(summary.latency_p50_ms, Some(1_000));
        assert_eq!(summary.latency_p95_ms, Some(1_900));
        assert_eq!(summary.peak_resident_memory_bytes, Some(20 * 1024));
        assert_eq!(summary.peak_disk_bytes, 64);
    }

    #[test]
    fn budget_requires_metrics_instead_of_passing_missing_values() {
        let verdict =
            PerformanceSummary::from_samples(&[]).evaluate(PerformancePlatform::Linux.budget());
        assert!(!verdict.passed);
        assert_eq!(
            verdict.violations,
            vec![
                BudgetViolation::MissingLatency,
                BudgetViolation::MissingMemory
            ]
        );
    }

    #[test]
    fn budget_reports_each_exceeded_limit() {
        let budget = PerformancePlatform::Linux.budget();
        let summary = PerformanceSummary::from_samples(&[
            metric_sample(Some(31_000), Some(budget.resident_memory_bytes + 1)),
            metric_sample(Some(11_000), Some(1024)),
        ]);
        let verdict = summary.evaluate(budget);

        assert!(!verdict.passed);
        assert_eq!(
            verdict.violations,
            vec![
                BudgetViolation::LatencyP50,
                BudgetViolation::LatencyP95,
                BudgetViolation::ResidentMemory
            ]
        );
    }

    #[test]
    fn platform_budgets_are_explicit_and_android_is_more_constrained() {
        let linux = PerformancePlatform::Linux.budget();
        let windows = PerformancePlatform::Windows.budget();
        let android = PerformancePlatform::Android.budget();

        assert_eq!(linux, windows);
        assert!(android.latency_p95_ms > linux.latency_p95_ms);
        assert!(android.resident_memory_bytes < linux.resident_memory_bytes);
    }
}
