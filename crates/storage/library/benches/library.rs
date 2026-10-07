//! Benchmarks of `open`, `build_search_index` and `list` with 1000 documents of 20 KB.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use antenna_core::{Document, TextFormat};
use antenna_library::{Filter, Library};
use divan::Bencher;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

const DOCUMENTS: i64 = 1_000;
const DOCUMENT_BYTES: usize = 20_000;
const SECONDS_PER_DAY: i64 = 86_400;
const FIRST_DOCUMENT_SECONDS: i64 = 1_700_000_000;
const NOW_SECONDS: i64 = 1_790_000_000;
const WORDS: [&str; 16] = [
    "casa",
    "mercado",
    "lluvia",
    "camino",
    "ventana",
    "cancion",
    "tiempo",
    "ciudad",
    "rio",
    "monta\u{f1}a",
    "caf\u{e9}",
    "libro",
    "puerta",
    "viaje",
    "nube",
    "jard\u{ed}n",
];
const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const LCG_INCREMENT: u64 = 1;
const LCG_HIGH_BITS_SHIFT: u32 = 33;
const QUERY: &str = "casa ausente";
const SAMPLE_COUNT: u32 = 10;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

fn main() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    populate(root.path())?;
    ROOT.get_or_init(|| root.path().to_owned());
    divan::main();
    Ok(())
}

fn populate(root: &Path) -> Result<(), Box<dyn Error>> {
    let library = Library::open(root)?;
    for index in 0..DOCUMENTS {
        let document = Document::new(text_of(index), TextFormat::Plain)?;
        let modified = Timestamp::from_second(FIRST_DOCUMENT_SECONDS + index * SECONDS_PER_DAY)?;
        library.create(&format!("Document {index}"), &document, modified)?;
    }
    Ok(())
}

fn text_of(index: i64) -> String {
    let mut text = String::with_capacity(DOCUMENT_BYTES + 16);
    let mut state = u64::try_from(index).unwrap_or_default() + 1;
    while text.len() < DOCUMENT_BYTES {
        state = state
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        let word = WORDS.get((state >> LCG_HIGH_BITS_SHIFT) as usize % WORDS.len());
        text.push_str(word.copied().unwrap_or_default());
        text.push(' ');
    }
    text
}

#[expect(
    clippy::expect_used,
    reason = "main sets the root before a benchmark runs"
)]
fn root() -> &'static Path {
    ROOT.get()
        .expect("main sets the root before the benchmarks run")
}

#[expect(clippy::expect_used, reason = "the constant is a valid timestamp")]
fn now() -> Zoned {
    let timestamp = Timestamp::from_second(NOW_SECONDS).expect("the constant is in range");
    timestamp.to_zoned(TimeZone::UTC)
}

#[expect(
    clippy::expect_used,
    reason = "a benchmark with no library has no result to report"
)]
fn open_root() -> Library {
    Library::open(root()).expect("the root holds a library")
}

#[expect(
    clippy::expect_used,
    reason = "a benchmark with no index has no result to report"
)]
fn build_index(library: &Library) {
    library.build_search_index().expect("the text files exist");
}

#[divan::bench(sample_count = SAMPLE_COUNT)]
fn open_1000() -> Library {
    open_root()
}

#[divan::bench(sample_count = SAMPLE_COUNT)]
fn index_1000(bencher: Bencher<'_, '_>) {
    bencher
        .with_inputs(open_root)
        .bench_refs(|library| build_index(library));
}

#[divan::bench(sample_count = SAMPLE_COUNT)]
fn search_1000(bencher: Bencher<'_, '_>) {
    let library = open_root();
    build_index(&library);
    let now = now();

    bencher.bench(|| library.list(Filter::All, QUERY, &now));
}
