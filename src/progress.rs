use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ProgressStats {
    files_processed: usize,
    bytes_processed: u64,
    matches_found: usize,
    start_time: Instant,
}

pub struct ProgressReporter {
    pub multi_progress: Arc<MultiProgress>,
    pub main_progress: ProgressBar,
    pub stats: Arc<Mutex<ProgressStats>>,
    pub total_files: usize,
    pub style: ProgressStyle,
}

impl ProgressReporter {
    pub fn new(total_files: usize) -> Self {
        let multi_progress = Arc::new(MultiProgress::new());
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} files ({eta})\n{msg}")
            .unwrap()
            .progress_chars("=>-");

        let main_progress = multi_progress.add(ProgressBar::new(total_files as u64));
        main_progress.set_style(style.clone());

        Self {
            multi_progress,
            main_progress,
            stats: Arc::new(Mutex::new(ProgressStats {
                files_processed: 0,
                bytes_processed: 0,
                matches_found: 0,
                start_time: Instant::now(),
            })),
            total_files,
            style,
        }
    }

    pub fn create_subprocess_bar(&self, name: &str, size: u64) -> ProgressBar {
        let pb = ProgressBar::new(size);
        pb.set_style(self.style.clone());
        pb.set_prefix(name.to_string());
        self.multi_progress.add(pb)
    }

    pub fn update(&self, files: usize, bytes: u64, matches: usize) {
        let mut stats = self.stats.lock();
        stats.files_processed += files;
        stats.bytes_processed += bytes;
        stats.matches_found += matches;

        self.main_progress.inc(files as u64);
        self.update_message(&stats);
    }

    fn update_message(&self, stats: &ProgressStats) {
        let elapsed = stats.start_time.elapsed();
        let speed = if elapsed.as_secs() > 0 {
            stats.bytes_processed as f64 / elapsed.as_secs() as f64
        } else {
            0.0
        };

        self.main_progress.set_message(format!(
            "Speed: {:.2} MB/s | Matches: {} | Total Size: {:.2} MB",
            speed / (1024.0 * 1024.0),
            stats.matches_found,
            stats.bytes_processed as f64 / (1024.0 * 1024.0)
        ));
    }

    pub fn finish(self) -> ProgressStats {
        let stats = self.stats.lock().clone();
        self.main_progress.finish_with_message(format!(
            "Completed in {:.2}s: {} files, {} matches",
            stats.start_time.elapsed().as_secs_f64(),
            stats.files_processed,
            stats.matches_found
        ));
        stats
    }
}
