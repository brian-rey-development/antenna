use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;

use cpal::{FromSample, Sample};
use rtrb::Consumer;

use super::atomics::PlayerAtomics;

/// The realtime function of the output stream. It does not allocate, lock or do I/O.
pub(crate) fn fill<T>(
    output: &mut [T],
    channels: NonZeroUsize,
    consumer: &mut Consumer<f32>,
    atomics: &PlayerAtomics,
) where
    T: Sample + FromSample<f32>,
{
    // The flush comes first, so a seek works while the track is paused.
    discard_when_flush_requested(consumer, atomics);
    if atomics.paused.load(Ordering::Relaxed) {
        output.fill(T::EQUILIBRIUM);
        return;
    }
    let channels = channels.get();
    let volume = f32::from_bits(atomics.volume_bits.load(Ordering::Relaxed));
    let popped = pop_frames(output, channels, consumer, volume);
    for frame in output.chunks_exact_mut(channels).skip(popped) {
        frame.fill(T::EQUILIBRIUM);
    }
    atomics
        .played_frames
        .fetch_add(popped as u64, Ordering::Release);
    let is_short = popped < output.len() / channels;
    if is_short && !atomics.is_buffering.load(Ordering::Relaxed) {
        atomics.underruns.fetch_add(1, Ordering::Relaxed);
    }
}

fn discard_when_flush_requested(consumer: &mut Consumer<f32>, atomics: &PlayerAtomics) {
    let epoch = atomics.flush_epoch.load(Ordering::Acquire);
    if epoch <= atomics.flushed_epoch.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(chunk) = consumer.read_chunk(consumer.slots()) {
        chunk.commit_all();
    }
    let played = atomics.played_frames.load(Ordering::Relaxed);
    atomics.flushed_at_frame.store(played, Ordering::Relaxed);
    atomics.flushed_epoch.store(epoch, Ordering::Release);
}

fn pop_frames<T>(
    output: &mut [T],
    channels: usize,
    consumer: &mut Consumer<f32>,
    volume: f32,
) -> usize
where
    T: Sample + FromSample<f32>,
{
    let wanted = consumer.slots().min(output.len() / channels);
    let Ok(chunk) = consumer.read_chunk(wanted) else {
        return 0;
    };
    let (head, tail) = chunk.as_slices();
    let samples = head.iter().chain(tail);
    for (frame, sample) in output.chunks_exact_mut(channels).zip(samples) {
        frame.fill(T::from_sample(sample * volume));
    }
    chunk.commit_all();
    wanted
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;
    use rtrb::{Producer, RingBuffer};

    use super::*;
    use crate::test_support::assert_same;

    const CAPACITY: usize = 64;

    fn setup(samples: &[f32]) -> (Producer<f32>, Consumer<f32>, PlayerAtomics) {
        let (mut producer, consumer) = RingBuffer::new(CAPACITY);
        producer.push_entire_slice(samples).unwrap();
        let atomics = PlayerAtomics::new();
        atomics.paused.store(false, Ordering::Relaxed);
        (producer, consumer, atomics)
    }

    fn channels(count: usize) -> NonZeroUsize {
        NonZeroUsize::new(count).unwrap()
    }

    fn played(atomics: &PlayerAtomics) -> u64 {
        atomics.played_frames.load(Ordering::Relaxed)
    }

    fn underruns(atomics: &PlayerAtomics) -> u64 {
        atomics.underruns.load(Ordering::Relaxed)
    }

    fn assert_allocation_free(atomics: &PlayerAtomics, mut consumer: Consumer<f32>) {
        let mut output = [0.0_f32; 16];

        assert_no_alloc(|| fill(&mut output, channels(2), &mut consumer, atomics));
    }

    #[test]
    fn fill_copies_sample_to_each_channel() {
        let (_producer, mut consumer, atomics) = setup(&[0.1, 0.2, 0.3]);
        let mut output = [9.0_f32; 6];

        fill(&mut output, channels(2), &mut consumer, &atomics);

        assert_same(&output, &[0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
        assert_eq!(played(&atomics), 3);
    }

    #[test]
    fn fill_applies_volume() {
        let (_producer, mut consumer, atomics) = setup(&[0.5, -1.0]);
        atomics
            .volume_bits
            .store(0.5_f32.to_bits(), Ordering::Relaxed);
        let mut output = [9.0_f32; 2];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_same(&output, &[0.25, -0.5]);
    }

    #[test]
    fn fill_converts_samples_when_output_is_i16() {
        let (_producer, mut consumer, atomics) = setup(&[1.0, -1.0]);
        let mut output = [7_i16; 3];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_eq!(output, [i16::MAX, i16::MIN, 0]);
    }

    #[test]
    fn fill_writes_silence_when_paused() {
        let (_producer, mut consumer, atomics) = setup(&[0.5, 0.5]);
        atomics.paused.store(true, Ordering::Relaxed);
        let mut output = [9.0_f32; 4];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_same(&output, &[0.0; 4]);
        assert_eq!(played(&atomics), 0);
        assert_eq!(consumer.slots(), 2);
        assert_eq!(underruns(&atomics), 0);
    }

    #[test]
    fn fill_discards_samples_when_flush_requested() {
        let (_producer, mut consumer, atomics) = setup(&[0.5; 8]);
        atomics.played_frames.store(40, Ordering::Relaxed);
        atomics.flush_epoch.store(3, Ordering::Relaxed);
        let mut output = [9.0_f32; 2];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_eq!(atomics.flushed_epoch.load(Ordering::Relaxed), 3);
        assert_eq!(atomics.flushed_at_frame.load(Ordering::Relaxed), 40);
        assert_same(&output, &[0.0; 2]);
        assert_eq!(played(&atomics), 40);
    }

    #[test]
    fn fill_discards_samples_when_flush_requested_while_paused() {
        let (_producer, mut consumer, atomics) = setup(&[0.5; 8]);
        atomics.paused.store(true, Ordering::Relaxed);
        atomics.flush_epoch.store(1, Ordering::Relaxed);
        let mut output = [9.0_f32; 2];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_eq!(atomics.flushed_epoch.load(Ordering::Relaxed), 1);
        assert_eq!(consumer.slots(), 0);
        assert_eq!(played(&atomics), 0);
    }

    #[test]
    fn fill_counts_underrun_when_playing_and_short() {
        let (_producer, mut consumer, atomics) = setup(&[0.5]);
        let mut output = [9.0_f32; 4];

        fill(&mut output, channels(1), &mut consumer, &atomics);
        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_same(&output, &[0.0; 4]);
        assert_eq!(underruns(&atomics), 2);
        assert_eq!(played(&atomics), 1);
    }

    #[test]
    fn fill_counts_no_underrun_when_buffer_fills_output() {
        let (_producer, mut consumer, atomics) = setup(&[0.5; 4]);
        let mut output = [9.0_f32; 4];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_eq!(underruns(&atomics), 0);
    }

    #[test]
    fn fill_skips_underrun_when_buffering() {
        let (_producer, mut consumer, atomics) = setup(&[]);
        atomics.is_buffering.store(true, Ordering::Relaxed);
        let mut output = [9.0_f32; 4];

        fill(&mut output, channels(1), &mut consumer, &atomics);

        assert_same(&output, &[0.0; 4]);
        assert_eq!(underruns(&atomics), 0);
    }

    #[test]
    fn fill_does_not_allocate_when_normal() {
        let (_producer, consumer, atomics) = setup(&[0.5; 8]);

        assert_allocation_free(&atomics, consumer);

        assert_eq!(played(&atomics), 8);
    }

    #[test]
    fn fill_does_not_allocate_when_paused() {
        let (_producer, consumer, atomics) = setup(&[0.5; 8]);
        atomics.paused.store(true, Ordering::Relaxed);

        assert_allocation_free(&atomics, consumer);

        assert_eq!(played(&atomics), 0);
    }

    #[test]
    fn fill_does_not_allocate_when_flush() {
        let (_producer, consumer, atomics) = setup(&[0.5; 8]);
        atomics.flush_epoch.store(1, Ordering::Relaxed);

        assert_allocation_free(&atomics, consumer);

        assert_eq!(atomics.flushed_epoch.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn fill_does_not_allocate_when_underrun() {
        let (_producer, consumer, atomics) = setup(&[0.5; 2]);

        assert_allocation_free(&atomics, consumer);

        assert_eq!(underruns(&atomics), 1);
    }

    #[test]
    fn fill_does_not_allocate_when_buffering() {
        let (_producer, consumer, atomics) = setup(&[0.5; 2]);
        atomics.is_buffering.store(true, Ordering::Relaxed);

        assert_allocation_free(&atomics, consumer);

        assert_eq!(underruns(&atomics), 0);
    }
}
