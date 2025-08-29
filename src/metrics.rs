use prometheus::{Encoder, IntCounter, Opts, Registry, TextEncoder};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    pub files_scanned: IntCounter,
    pub matches_found: IntCounter,
    pub files_skipped: IntCounter,
    pub worker_timeouts: IntCounter,
    registry: Arc<Registry>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let files_scanned =
            IntCounter::with_opts(Opts::new("files_scanned", "Number of files scanned")).unwrap();
        let matches_found =
            IntCounter::with_opts(Opts::new("matches_found", "Number of matches found")).unwrap();
        let files_skipped = IntCounter::with_opts(Opts::new(
            "files_skipped",
            "Number of files skipped by heuristics",
        ))
        .unwrap();
        let worker_timeouts =
            IntCounter::with_opts(Opts::new("worker_timeouts", "Number of worker timeouts"))
                .unwrap();

        registry.register(Box::new(files_scanned.clone())).ok();
        registry.register(Box::new(matches_found.clone())).ok();
        registry.register(Box::new(files_skipped.clone())).ok();
        registry.register(Box::new(worker_timeouts.clone())).ok();

        Metrics {
            files_scanned,
            matches_found,
            files_skipped,
            worker_timeouts,
            registry: Arc::new(registry),
        }
    }

    pub fn gather(&self) -> String {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap_or_default()
    }
}
