use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::ModelError;

pub(crate) const RETRY_COUNT: usize = 3;
pub(crate) const RETRY_DELAYS: [Duration; RETRY_COUNT] = [
    Duration::from_secs(1),
    Duration::from_secs(4),
    Duration::from_secs(16),
];
pub(crate) const CANCEL_POLL: Duration = Duration::from_millis(100);

/// Returns [`ModelError::Cancelled`] if the cancel flag is `true`.
pub(crate) fn check(cancel: &AtomicBool) -> Result<(), ModelError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(ModelError::Cancelled);
    }
    Ok(())
}

/// Waits for the delay. Returns [`ModelError::Cancelled`] as soon as the cancel flag is `true`.
pub(crate) fn wait(delay: Duration, cancel: &AtomicBool) -> Result<(), ModelError> {
    let end = Instant::now() + delay;
    loop {
        check(cancel)?;
        let remaining = end.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        thread::park_timeout(remaining.min(CANCEL_POLL));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_returns_cancelled_when_flag_is_set() {
        let cancel = AtomicBool::new(true);

        let result = wait(Duration::from_secs(60), &cancel);

        assert!(matches!(result, Err(ModelError::Cancelled)));
    }

    #[test]
    fn wait_returns_cancelled_when_delay_is_zero_and_flag_is_set() {
        let cancel = AtomicBool::new(true);

        let result = wait(Duration::ZERO, &cancel);

        assert!(matches!(result, Err(ModelError::Cancelled)));
    }

    #[test]
    fn wait_returns_ok_when_delay_is_zero() {
        let cancel = AtomicBool::new(false);

        let result = wait(Duration::ZERO, &cancel);

        assert!(result.is_ok());
    }
}
