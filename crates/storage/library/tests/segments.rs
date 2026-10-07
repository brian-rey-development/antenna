//! Tests of the segment keys, the segment writer and the segment store.

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::num::NonZeroUsize;
    use std::path::Path;
    use std::time::Duration;

    use antenna_core::{
        EngineDescriptor, EngineId, Localized, ModelLicense, PCM_SCALE, Quality, SampleRate,
        Variant, Variants, VoiceId,
    };
    use antenna_library::{Library, LibraryError, SegmentKey, SegmentStore};
    use tempfile::TempDir;

    const NO_FILES: Variant = Variant {
        parameters: "0",
        artifacts: &[],
    };

    const fn descriptor(id: EngineId, version: &'static str) -> EngineDescriptor {
        EngineDescriptor {
            id,
            name: "Fake",
            version,
            summary: Localized { en: "", es: "" },
            license: ModelLicense {
                name: "MIT",
                url: "",
                attribution: "",
            },
            sample_rate: SampleRate::HZ_24000,
            max_segment_chars: NonZeroUsize::MIN,
            variants: Variants {
                fast: NO_FILES,
                balanced: NO_FILES,
                max: NO_FILES,
            },
            voices: &[],
        }
    }

    const FAKE: EngineId = EngineId::new("fake");
    const OTHER: EngineId = EngineId::new("other");
    const VOICE: VoiceId = VoiceId::new(FAKE, "en-alba");
    const RATE: SampleRate = SampleRate::HZ_24000;

    fn store() -> (TempDir, SegmentStore) {
        let root = tempfile::tempdir().unwrap();
        let store = SegmentStore::open(root.path()).unwrap();
        (root, store)
    }

    fn key(digit: char) -> SegmentKey {
        digit.to_string().repeat(64).parse().unwrap()
    }

    fn files_under(directory: &Path) -> Vec<String> {
        let mut names = Vec::new();
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                names.extend(files_under(&path));
            } else {
                names.push(path.display().to_string());
            }
        }
        names
    }

    fn store_segment(store: &SegmentStore, key: SegmentKey, samples: &[f32]) -> Duration {
        let mut writer = store.writer(key, RATE).unwrap();
        writer.write(samples).unwrap();
        writer.commit().unwrap()
    }

    #[test]
    fn segment_key_matches_reference_digest_when_fields_given() {
        let key = SegmentKey::new(&descriptor(FAKE, "1"), VOICE, Quality::Balanced, "Hello.");

        assert_eq!(
            key.to_string(),
            "e6c843580522f378e646ffd8ba1c773380b52a0fd3e66297dcc1868776919cfc"
        );
    }

    #[test]
    fn segment_key_changes_when_any_field_changes() {
        let other_voice = VoiceId::new(FAKE, "en-bruno");
        let other_engine_voice = VoiceId::new(OTHER, "en-alba");
        let keys = [
            SegmentKey::new(&descriptor(FAKE, "1"), VOICE, Quality::Balanced, "Hello."),
            SegmentKey::new(
                &descriptor(OTHER, "1"),
                other_engine_voice,
                Quality::Balanced,
                "Hello.",
            ),
            SegmentKey::new(&descriptor(FAKE, "2"), VOICE, Quality::Balanced, "Hello."),
            SegmentKey::new(
                &descriptor(FAKE, "1"),
                other_voice,
                Quality::Balanced,
                "Hello.",
            ),
            SegmentKey::new(&descriptor(FAKE, "1"), VOICE, Quality::Max, "Hello."),
            SegmentKey::new(&descriptor(FAKE, "1"), VOICE, Quality::Balanced, "Hello!"),
        ];

        let distinct: HashSet<SegmentKey> = keys.into_iter().collect();

        assert_eq!(distinct.len(), keys.len());
    }

    #[test]
    fn segment_key_differs_when_text_moves_between_fields() {
        let short_version = descriptor(FAKE, "1");
        let long_version = descriptor(FAKE, "12");

        let first = SegmentKey::new(&short_version, VOICE, Quality::Fast, "23");
        let second = SegmentKey::new(&long_version, VOICE, Quality::Fast, "3");

        assert_ne!(first, second);
    }

    #[test]
    fn segment_key_round_trips_when_text_parsed() {
        let text = format!("0aff{}", "5".repeat(60));

        let key: SegmentKey = text.parse().unwrap();

        assert_eq!(key.to_string(), text);
    }

    #[test]
    fn segment_key_fails_when_text_not_64_hex_characters() {
        let invalid = ["", "abc", &"a".repeat(65), &"A".repeat(64), &"g".repeat(64)];

        for text in invalid {
            let result = text.parse::<SegmentKey>();

            assert!(
                matches!(&result, Err(LibraryError::InvalidSegmentKey { text: kept }) if kept == text),
                "{text:?}"
            );
        }
    }

    #[test]
    fn segment_round_trips_when_written_and_read() {
        let (_root, store) = store();
        let samples: Vec<f32> = (-100..=100_i16)
            .map(|step| f32::from(step) / 100.0)
            .collect();

        let duration = store_segment(&store, key('a'), &samples);
        let stored = store.read(&key('a')).unwrap();

        assert_eq!(stored.rate, RATE);
        assert_eq!(stored.samples.len(), samples.len());
        for (read, written) in stored.samples.iter().zip(&samples) {
            assert!(
                (read - written).abs() <= 1.0 / PCM_SCALE,
                "{read} vs {written}"
            );
        }
        assert_eq!(duration, RATE.duration_of(samples.len() as u64));
        assert_eq!(store.duration(&key('a')).unwrap(), duration);
    }

    #[test]
    fn segment_stores_silence_when_no_sample_written() {
        let (_root, store) = store();

        let duration = store_segment(&store, key('b'), &[]);

        assert_eq!(duration, Duration::ZERO);
        assert!(store.read(&key('b')).unwrap().samples.is_empty());
    }

    #[test]
    fn segment_absent_until_commit() {
        let (_root, store) = store();
        let mut writer = store.writer(key('c'), RATE).unwrap();
        writer.write(&[0.5; 64]).unwrap();

        let before = store.contains(&key('c'));
        writer.commit().unwrap();

        assert!(!before);
        assert!(store.contains(&key('c')));
        assert!(store.path(&key('c')).is_file());
    }

    #[test]
    fn dropped_writer_deletes_temp_file() {
        let (root, store) = store();
        let mut writer = store.writer(key('d'), RATE).unwrap();
        writer.write(&[0.5; 64]).unwrap();

        drop(writer);

        assert!(!store.contains(&key('d')));
        assert_eq!(files_under(root.path()), Vec::<String>::new());
    }

    #[test]
    fn commit_keeps_existing_file_when_key_exists() {
        let (root, store) = store();
        store_segment(&store, key('e'), &[0.25; 10]);

        let duration = store_segment(&store, key('e'), &[-0.25; 20]);

        let stored = store.read(&key('e')).unwrap();
        assert_eq!(stored.samples.len(), 10);
        assert_eq!(duration, RATE.duration_of(20));
        assert_eq!(files_under(root.path()).len(), 1);
    }

    #[test]
    fn read_fails_when_segment_absent() {
        let (_root, store) = store();

        let result = store.read(&key('f'));

        assert!(matches!(result, Err(LibraryError::Wav { .. })));
    }

    #[test]
    fn open_makes_segments_directory_when_missing() {
        let root = tempfile::tempdir().unwrap();

        SegmentStore::open(root.path().join("nested")).unwrap();

        assert!(root.path().join("nested").join("segments").is_dir());
    }

    #[test]
    fn segment_store_matches_library_store_when_root_same() {
        let root = tempfile::tempdir().unwrap();
        let library = Library::open(root.path()).unwrap();
        let store = SegmentStore::open(root.path()).unwrap();

        store_segment(library.segments(), key('9'), &[0.5; 4]);

        assert_eq!(store.path(&key('9')), library.segments().path(&key('9')));
        assert!(store.contains(&key('9')));
    }
}
