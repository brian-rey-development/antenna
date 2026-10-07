use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::AudioError;
use crate::segment_file::SegmentFile;

/// The number of waveform peaks for each second of audio.
pub const PEAKS_PER_SECOND: u32 = 50;

/// The waveform peaks of stored segments. A stored segment never changes, so the cache never
/// removes an entry.
#[derive(Debug, Default)]
pub struct PeakCache {
    entries: HashMap<PathBuf, Arc<[f32]>>,
}

impl PeakCache {
    /// Returns the peaks of a stored segment. The first call for a path reads the file. A peak is
    /// the largest absolute sample of a bucket, and a bucket has `rate / PEAKS_PER_SECOND`
    /// samples, rounded up.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError::ReadSegment`] or [`AudioError::UnsupportedSegment`] if the file is
    /// not a readable 16-bit mono WAV file.
    pub fn peaks(&mut self, path: &Path) -> Result<Arc<[f32]>, AudioError> {
        if let Some(peaks) = self.entries.get(path) {
            return Ok(Arc::clone(peaks));
        }
        let peaks: Arc<[f32]> = read_peaks(path)?.into();
        self.entries.insert(path.to_owned(), Arc::clone(&peaks));
        Ok(peaks)
    }
}

fn read_peaks(path: &Path) -> Result<Vec<f32>, AudioError> {
    let file = SegmentFile::open(path)?;
    let bucket = file.rate().hz().div_ceil(PEAKS_PER_SECOND) as usize;
    let samples = file.into_samples()?;
    Ok(samples
        .chunks(bucket)
        .map(|chunk| {
            chunk
                .iter()
                .fold(0.0, |peak, sample| f32::max(peak, sample.abs()))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use antenna_core::SampleRate;

    use super::*;
    use crate::test_support::{pcm_exact, write_segment};

    const BUCKET: usize = 24_000 / PEAKS_PER_SECOND as usize;

    #[test]
    fn peaks_match_bucket_maximums() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        let mut samples = vec![0.0; 2 * 24_000];
        samples[10] = -0.5;
        samples[BUCKET + 7] = 0.25;
        samples[BUCKET + 8] = -0.75;
        samples[24_000 + 3] = 1.0;
        write_segment(&path, SampleRate::HZ_24000, &samples).unwrap();

        let peaks = PeakCache::default().peaks(&path).unwrap();

        let [half, three_quarters] = pcm_exact(&[0.5, 0.75])[..] else {
            unreachable!("two samples in, two samples out")
        };
        let mut expected = vec![0.0; 2 * PEAKS_PER_SECOND as usize];
        expected[0] = half;
        expected[1] = three_quarters;
        expected[PEAKS_PER_SECOND as usize] = 1.0;
        assert_eq!(peaks.to_vec(), expected);
    }

    #[test]
    fn peaks_keep_partial_bucket_when_length_not_multiple() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        write_segment(&path, SampleRate::HZ_24000, &vec![0.5; BUCKET + 1]).unwrap();

        let peaks = PeakCache::default().peaks(&path).unwrap();

        assert_eq!(peaks.len(), 2);
    }

    #[test]
    fn peaks_are_empty_when_file_has_no_samples() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        write_segment(&path, SampleRate::HZ_24000, &[]).unwrap();

        let peaks = PeakCache::default().peaks(&path).unwrap();

        assert!(peaks.is_empty());
    }

    #[test]
    fn peaks_round_bucket_up_when_rate_not_multiple_of_50() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        write_segment(&path, SampleRate::HZ_22050, &vec![0.5; 22_050 + 1]).unwrap();

        let peaks = PeakCache::default().peaks(&path).unwrap();

        assert_eq!(peaks.len(), 51);
    }

    #[test]
    fn peaks_reuse_entry_when_called_again() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.wav");
        write_segment(&path, SampleRate::HZ_24000, &[0.5; 100]).unwrap();
        let mut cache = PeakCache::default();
        let first = cache.peaks(&path).unwrap();
        fs::remove_file(&path).unwrap();

        let second = cache.peaks(&path).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn peaks_fail_when_file_missing() {
        let directory = tempfile::tempdir().unwrap();

        let result = PeakCache::default().peaks(&directory.path().join("none.wav"));

        assert!(matches!(result, Err(AudioError::ReadSegment { .. })));
    }

    #[test]
    fn peaks_read_file_again_when_first_read_failed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("late.wav");
        let mut cache = PeakCache::default();
        assert!(cache.peaks(&path).is_err());
        write_segment(&path, SampleRate::HZ_24000, &[1.0]).unwrap();

        let peaks = cache.peaks(&path).unwrap();

        assert_eq!(peaks.to_vec(), [1.0]);
    }
}
