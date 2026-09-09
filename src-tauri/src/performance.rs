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
}
