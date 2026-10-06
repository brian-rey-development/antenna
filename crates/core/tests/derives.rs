//! Compile-time checks of the derives of the core types.

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt::{Debug, Display};
    use std::hash::Hash;
    use std::str::FromStr;

    use antenna_core::{
        Artifact, CoreError, Defect, Document, Engine, EngineDescriptor, EngineError,
        EngineFactory, EngineId, ExportFormat, Extent, Language, Localized, ModelFiles,
        ModelLicense, Output, PcmChunk, Quality, SampleRate, Segment, SegmentIndex, Sink,
        SinkError, TextFormat, TextHash, Variant, Variants, VoiceDescriptor, VoiceId,
    };

    fn text_form<T>()
    where
        T: Copy
            + Debug
            + Eq
            + Hash
            + Display
            + FromStr<Err = strum::ParseError>
            + Into<&'static str>,
    {
    }

    fn ordered<T: Copy + Debug + Eq + Hash + Ord>() {}

    fn defaulted<T: Default>() {}

    fn displayed<T: Display>() {}

    fn copied<T: Copy + Debug + Eq>() {}

    fn compared<T: Clone + Debug + Eq>() {}

    fn partially_compared<T: Clone + Debug + PartialEq>() {}

    fn parsed<T: FromStr<Err = CoreError>>() {}

    fn converted<T: TryFrom<u32, Error = CoreError>>() {}

    fn indexed<T: Into<usize>>() {}

    fn hashed<T: Clone + Debug + Eq + Hash>() {}

    fn error<T: Error + Send + Sync + 'static>() {}

    fn sent<T: Send + ?Sized>() {}

    macro_rules! bounded {
        ($check:ident: $($type:ty),+) => {
            $($check::<$type>();)+
        };
    }

    #[test]
    fn core_types_have_required_derives() {
        bounded!(text_form: Language, Quality, ExportFormat, TextFormat);
        bounded!(ordered: Language, Quality, EngineId, VoiceId, SampleRate, SegmentIndex);
        bounded!(defaulted: Quality, ExportFormat, ModelFiles);
        bounded!(displayed: EngineId, VoiceId, SegmentIndex, TextHash);
        bounded!(hashed: Localized, TextHash, Document, Segment);
        bounded!(compared: ModelFiles);
        bounded!(partially_compared: PcmChunk);
        bounded!(parsed: TextHash);
        bounded!(converted: SampleRate);
        bounded!(indexed: SegmentIndex);
        bounded!(copied: Localized, TextHash, ModelLicense, Artifact, Extent, Variant);
        bounded!(copied: Variants, VoiceDescriptor, EngineDescriptor, Defect);
        bounded!(error: CoreError, EngineError, SinkError);
        bounded!(sent: dyn Engine, dyn Output, dyn Sink, dyn EngineFactory);
    }
}
