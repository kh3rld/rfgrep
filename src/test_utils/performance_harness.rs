//! Performance testing harness for rfgrep
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance measurement result
#[derive(Debug, Clone)]
pub struct Measurement {
    pub name: String,
    pub duration: Duration,
    pub iterations: usize,
    pub throughput: f64,
}

/// Performance test harness with advanced statistics
pub struct PerformanceHarness {
    start_time: Instant,
    measurements: Vec<Measurement>,
    iteration_counts: HashMap<String, usize>,
}

impl PerformanceHarness {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            measurements: Vec::new(),
            iteration_counts: HashMap::new(),
        }
    }

    /// Measure a single operation
    pub fn measure<F, R>(&mut self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.measure_iterations(name, 1, f)
    }

    /// Measure multiple iterations of an operation
    pub fn measure_iterations<F, R>(&mut self, name: &str, iterations: usize, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        let throughput = if duration.as_secs_f64() > 0.0 {
            iterations as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let measurement = Measurement {
            name: name.to_string(),
            duration,
            iterations,
            throughput,
        };

        self.measurements.push(measurement);
        *self.iteration_counts.entry(name.to_string()).or_insert(0) += iterations;
        result
    }

    /// Measure with automatic iteration count to reach target duration
    pub fn measure_auto_iterations<F, R>(
        &mut self,
        name: &str,
        target_duration: Duration,
        f: F,
    ) -> R
    where
        F: Fn() -> R,
    {
        let mut iterations = 1;
        let mut total_duration = Duration::ZERO;

        let _ = f();

        while total_duration < target_duration && iterations < 1000000 {
            let start = Instant::now();
            let _ = f();
            total_duration += start.elapsed();
            iterations *= 2;
        }

        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        let throughput = if duration.as_secs_f64() > 0.0 {
            iterations as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let measurement = Measurement {
            name: name.to_string(),
            duration,
            iterations,
            throughput,
        };

        self.measurements.push(measurement);
        *self.iteration_counts.entry(name.to_string()).or_insert(0) += iterations;
        result
    }

    pub fn get_measurements(&self) -> &[Measurement] {
        &self.measurements
    }

    pub fn get_measurement(&self, name: &str) -> Option<&Measurement> {
        self.measurements.iter().find(|m| m.name == name)
    }

    pub fn total_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> PerformanceStats {
        let mut total_duration = Duration::ZERO;
        let mut total_throughput = 0.0;
        let mut operation_count = 0;

        for measurement in &self.measurements {
            total_duration += measurement.duration;
            total_throughput += measurement.throughput;
            operation_count += measurement.iterations;
        }

        let avg_throughput = if !self.measurements.is_empty() {
            total_throughput / self.measurements.len() as f64
        } else {
            0.0
        };

        PerformanceStats {
            total_duration,
            total_operations: operation_count,
            average_throughput: avg_throughput,
            measurement_count: self.measurements.len(),
        }
    }

    /// Generate a performance report
    pub fn generate_report(&self) -> String {
        let stats = self.get_stats();
        let mut report = String::new();

        report.push_str("Performance Report\n");
        report.push_str("==================\n");
        report.push_str(&format!("Total Duration: {:?}\n", stats.total_duration));
        report.push_str(&format!("Total Operations: {}\n", stats.total_operations));
        report.push_str(&format!(
            "Average Throughput: {:.2} ops/sec\n",
            stats.average_throughput
        ));
        report.push_str(&format!("Measurements: {}\n\n", stats.measurement_count));

        report.push_str("Individual Measurements:\n");
        report.push_str("----------------------\n");

        for measurement in &self.measurements {
            report.push_str(&format!(
                "{}: {:?} ({} iterations, {:.2} ops/sec)\n",
                measurement.name,
                measurement.duration,
                measurement.iterations,
                measurement.throughput
            ));
        }

        report
    }
}

/// Performance statistics summary
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_duration: Duration,
    pub total_operations: usize,
    pub average_throughput: f64,
    pub measurement_count: usize,
}

impl Default for PerformanceHarness {
    fn default() -> Self {
        Self::new()
    }
}
