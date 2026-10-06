use crate::{SampleRate, SegmentIndex, SinkError};

/// An object that the pipeline opens with the engine sample rate to get a sink. This trait is an
/// extension point.
///
/// ```
/// use antenna_core::{Output, SampleRate, SegmentIndex, Sink, SinkError};
///
/// struct Discard;
///
/// impl Output for Discard {
///     fn open(self: Box<Self>, rate: SampleRate) -> Result<Box<dyn Sink>, SinkError> {
///         if rate != SampleRate::HZ_24000 {
///             return Err(SinkError::RateMismatch { expected: SampleRate::HZ_24000, actual: rate });
///         }
///         Ok(self)
///     }
/// }
///
/// impl Sink for Discard {
///     fn begin_segment(&mut self, _: SegmentIndex) -> Result<(), SinkError> {
///         Ok(())
///     }
///
///     fn write(&mut self, _: &[f32]) -> Result<(), SinkError> {
///         Ok(())
///     }
///
///     fn finish(self: Box<Self>) -> Result<(), SinkError> {
///         Ok(())
///     }
/// }
///
/// assert!(Box::new(Discard).open(SampleRate::HZ_48000).is_err());
/// assert!(Box::new(Discard).open(SampleRate::HZ_24000).is_ok());
/// ```
pub trait Output: Send {
    /// Opens the output for the sample rate of the engine.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError::RateMismatch`] if the output cannot accept the rate, and
    /// [`SinkError::Closed`] if its receiver is gone.
    fn open(self: Box<Self>, rate: SampleRate) -> Result<Box<dyn Sink>, SinkError>;
}

/// An object that receives the samples of a job while the job runs. This trait is an extension
/// point.
///
/// ```
/// use antenna_core::{SegmentIndex, Sink, SinkError};
///
/// #[derive(Default)]
/// struct Count {
///     segments: usize,
///     samples: usize,
/// }
///
/// impl Sink for Count {
///     fn begin_segment(&mut self, _: SegmentIndex) -> Result<(), SinkError> {
///         self.segments += 1;
///         Ok(())
///     }
///
///     fn write(&mut self, samples: &[f32]) -> Result<(), SinkError> {
///         self.samples += samples.len();
///         Ok(())
///     }
///
///     fn finish(self: Box<Self>) -> Result<(), SinkError> {
///         Ok(())
///     }
/// }
///
/// let mut count = Count::default();
/// count.begin_segment(SegmentIndex::new(0)).unwrap();
/// count.write(&[0.0; 480]).unwrap();
/// assert_eq!((count.segments, count.samples), (1, 480));
/// Box::new(count).finish().unwrap();
/// ```
pub trait Sink: Send {
    /// Marks the start of a segment. The next samples belong to this segment.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError::Closed`] if the receiver of the sink is gone.
    fn begin_segment(&mut self, index: SegmentIndex) -> Result<(), SinkError>;

    /// Receives the next samples of the current segment.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError::Closed`] if the receiver of the sink is gone.
    fn write(&mut self, samples: &[f32]) -> Result<(), SinkError>;

    /// Marks the end of the job after the last segment.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError::Closed`] if the receiver of the sink is gone.
    fn finish(self: Box<Self>) -> Result<(), SinkError>;
}
