use std::cell::Cell;
use std::time::{Duration, Instant};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// The progress of the downloads of one request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadProgress {
    /// The bytes that the request downloaded so far.
    pub done_bytes: u64,
    /// The bytes that the request downloads in total.
    pub total_bytes: u64,
}

/// Joins the byte counts of the files of one request into throttled progress reports.
pub(crate) struct Reporter<'a> {
    callback: &'a (dyn Fn(DownloadProgress) + Sync),
    total_bytes: u64,
    completed_bytes: Cell<u64>,
    reported_bytes: Cell<u64>,
    last_report: Cell<Option<Instant>>,
}

impl<'a> Reporter<'a> {
    pub(crate) fn new(callback: &'a (dyn Fn(DownloadProgress) + Sync), total_bytes: u64) -> Self {
        Self {
            callback,
            total_bytes,
            completed_bytes: Cell::new(0),
            reported_bytes: Cell::new(0),
            last_report: Cell::new(None),
        }
    }

    /// Reports the bytes of the current file, unless the last report is less than one interval old.
    pub(crate) fn advance(&self, file_bytes: u64, now: Instant) {
        let is_recent = self
            .last_report
            .get()
            .is_some_and(|last| now.duration_since(last) < PROGRESS_INTERVAL);
        if is_recent {
            return;
        }
        self.report(self.completed_bytes.get().saturating_add(file_bytes));
        self.last_report.set(Some(now));
    }

    /// Adds the bytes of a finished file to the completed bytes.
    pub(crate) fn complete_file(&self, file_bytes: u64) {
        self.completed_bytes
            .set(self.completed_bytes.get().saturating_add(file_bytes));
    }

    /// Reports that the request is complete.
    pub(crate) fn finish(&self) {
        self.report(self.total_bytes);
    }

    fn report(&self, done_bytes: u64) {
        let done_bytes = done_bytes
            .min(self.total_bytes)
            .max(self.reported_bytes.get());
        self.reported_bytes.set(done_bytes);
        (self.callback)(DownloadProgress {
            done_bytes,
            total_bytes: self.total_bytes,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOTAL: u64 = 100;

    fn collect(run: impl FnOnce(&Reporter<'_>)) -> Vec<DownloadProgress> {
        let (sender, receiver) = flume::unbounded();
        let callback = move |progress| sender.send(progress).unwrap();
        let reporter = Reporter::new(&callback, TOTAL);
        run(&reporter);
        receiver.try_iter().collect()
    }

    fn at(start: Instant, milliseconds: u64) -> Instant {
        start + Duration::from_millis(milliseconds)
    }

    #[test]
    fn progress_is_monotonic_and_complete() {
        let start = Instant::now();

        let reports = collect(|reporter| {
            reporter.advance(30, at(start, 0));
            reporter.complete_file(40);
            reporter.advance(10, at(start, 200));
            reporter.advance(5, at(start, 400));
            reporter.complete_file(60);
            reporter.finish();
        });

        let done: Vec<u64> = reports.iter().map(|report| report.done_bytes).collect();
        assert_eq!(done, [30, 50, 50, 100]);
        assert!(reports.iter().all(|report| report.total_bytes == TOTAL));
    }

    #[test]
    fn progress_throttles_reports_when_interval_not_elapsed() {
        let start = Instant::now();

        let reports = collect(|reporter| {
            reporter.advance(10, at(start, 0));
            reporter.advance(20, at(start, 50));
            reporter.advance(30, at(start, 99));
            reporter.advance(40, at(start, 100));
            reporter.advance(50, at(start, 150));
            reporter.finish();
        });

        let done: Vec<u64> = reports.iter().map(|report| report.done_bytes).collect();
        assert_eq!(done, [10, 40, 100]);
    }

    #[test]
    fn progress_stays_at_total_when_download_restarts() {
        let start = Instant::now();

        let reports = collect(|reporter| {
            reporter.advance(TOTAL + 50, at(start, 0));
            reporter.finish();
        });

        let done: Vec<u64> = reports.iter().map(|report| report.done_bytes).collect();
        assert_eq!(done, [TOTAL, TOTAL]);
    }
}
