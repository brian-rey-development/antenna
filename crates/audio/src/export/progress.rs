use std::sync::atomic::{AtomicBool, Ordering};

use crate::AudioError;

/// The progress report and the cancel check of an export. Each pass over a segment is one step.
pub(crate) struct Progress<'a> {
    cancel: &'a AtomicBool,
    report: &'a dyn Fn(f32),
    done: usize,
    total: usize,
}

impl<'a> Progress<'a> {
    pub(crate) fn new(cancel: &'a AtomicBool, report: &'a dyn Fn(f32), total: usize) -> Self {
        Self {
            cancel,
            report,
            done: 0,
            total,
        }
    }

    pub(crate) fn complete_step(&mut self) -> Result<(), AudioError> {
        self.done += 1;
        (self.report)(fraction(self.done, self.total));
        if self.cancel.load(Ordering::Relaxed) {
            return Err(AudioError::Cancelled);
        }
        Ok(())
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the fraction is for display, and a step count below 2^24 is exact"
)]
fn fraction(done: usize, total: usize) -> f32 {
    done as f32 / total as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_reports_fraction_when_step_completes() {
        let cancel = AtomicBool::new(false);
        let (sender, receiver) = flume::unbounded();
        let report = move |value| sender.send(value).unwrap();
        let mut progress = Progress::new(&cancel, &report, 4);

        progress.complete_step().unwrap();
        progress.complete_step().unwrap();

        assert_eq!(receiver.drain().collect::<Vec<f32>>(), [0.25, 0.5]);
    }

    #[test]
    fn progress_fails_when_cancel_set() {
        let cancel = AtomicBool::new(false);
        let mut progress = Progress::new(&cancel, &|_| {}, 2);
        cancel.store(true, Ordering::Relaxed);

        let result = progress.complete_step();

        assert!(matches!(result, Err(AudioError::Cancelled)));
    }
}
