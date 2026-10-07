use std::mem;

use antenna_core::SampleRate;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use super::RESAMPLER_CHUNK_FRAMES;

const MONO: usize = 1;

#[derive(Debug)]
pub(crate) struct FftConversion {
    fft: Fft<f32>,
    pending: Vec<f32>,
    scratch: Vec<f32>,
    frames_to_skip: usize,
    received: u64,
    produced: u64,
    from_hz: u64,
    to_hz: u64,
}

impl FftConversion {
    #[expect(
        clippy::expect_used,
        reason = "the rates are not zero and the chunk size is not zero, the only errors of Fft::new"
    )]
    pub(super) fn new(from: SampleRate, to: SampleRate) -> Self {
        let (from_hz, to_hz) = (from.hz() as usize, to.hz() as usize);
        let fft = Fft::new(
            from_hz,
            to_hz,
            RESAMPLER_CHUNK_FRAMES,
            MONO,
            FixedSync::Input,
        )
        .expect("the sample rates and the chunk size are not zero");
        Self {
            frames_to_skip: fft.output_delay(),
            scratch: vec![0.0; fft.output_frames_max()],
            pending: Vec::with_capacity(2 * RESAMPLER_CHUNK_FRAMES),
            fft,
            received: 0,
            produced: 0,
            from_hz: u64::from(from.hz()),
            to_hz: u64::from(to.hz()),
        }
    }

    pub(super) fn convert(&mut self, input: &[f32], output: &mut Vec<f32>) {
        self.received += input.len() as u64;
        let mut rest = input;
        if !self.pending.is_empty() {
            let missing = RESAMPLER_CHUNK_FRAMES - self.pending.len();
            let (head, tail) = rest.split_at(missing.min(rest.len()));
            self.pending.extend_from_slice(head);
            rest = tail;
            if self.pending.len() < RESAMPLER_CHUNK_FRAMES {
                return;
            }
            self.process_pending(output);
        }
        let (chunks, remainder) = rest.as_chunks::<RESAMPLER_CHUNK_FRAMES>();
        for chunk in chunks {
            self.process_chunk(chunk, output);
        }
        self.pending.extend_from_slice(remainder);
    }

    /// Appends the samples that the delay of the resampler holds back. The output buffer must
    /// hold only the samples of this stream, because the end of the stream is trimmed from it.
    pub(super) fn finish(&mut self, output: &mut Vec<f32>) {
        let expected = (self.received * self.to_hz).div_ceil(self.from_hz);
        while self.produced < expected {
            self.pending.resize(RESAMPLER_CHUNK_FRAMES, 0.0);
            self.process_pending(output);
        }
        let excess = usize::try_from(self.produced - expected).unwrap_or(0);
        output.truncate(output.len().saturating_sub(excess));
        self.produced = expected;
    }

    fn process_pending(&mut self, output: &mut Vec<f32>) {
        let pending = mem::take(&mut self.pending);
        self.process_chunk(&pending, output);
        self.pending = pending;
        self.pending.clear();
    }

    #[expect(
        clippy::expect_used,
        reason = "the buffers have the sizes that the resampler asks for, the only errors of these calls"
    )]
    fn process_chunk(&mut self, chunk: &[f32], output: &mut Vec<f32>) {
        let input = InterleavedSlice::new(chunk, MONO, RESAMPLER_CHUNK_FRAMES)
            .expect("the chunk has the input frames");
        let frames_max = self.scratch.len();
        let mut buffer = InterleavedSlice::new_mut(&mut self.scratch, MONO, frames_max)
            .expect("the scratch has the output frames");
        let (_, frames) = self
            .fft
            .process_into_buffer(&input, &mut buffer, None)
            .expect("the resampler accepts a full chunk");
        let skipped = self.frames_to_skip.min(frames);
        self.frames_to_skip -= skipped;
        output.extend(self.scratch.iter().take(frames).skip(skipped));
        self.produced += (frames - skipped) as u64;
    }
}
