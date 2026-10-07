//! Benchmarks of `prepare` on documents of 1 MB.

use std::num::NonZeroUsize;

use antenna_core::{Document, Language, TextFormat};
use antenna_text::{SegmentLimits, prepare};

const DOCUMENT_BYTES: usize = 1_000_000;
const MAX_SEGMENT_CHARS: usize = 400;
const FIXTURE: &str = include_str!("../tests/fixtures/en.md");
const SHORT_BLOCK: &str = "- A short item.\n";

fn main() {
    divan::main();
}

#[expect(
    clippy::unwrap_used,
    reason = "a setup that fails must stop the benchmark, and the arguments are constants"
)]
fn bench_prepare(bencher: divan::Bencher<'_, '_>, unit: &str) {
    let text = unit.repeat(DOCUMENT_BYTES / unit.len() + 1);
    let document = Document::new(text, TextFormat::Markdown).unwrap();
    let limits = SegmentLimits::new(NonZeroUsize::new(MAX_SEGMENT_CHARS).unwrap());

    bencher.bench(|| prepare(&document, Language::En, limits));
}

#[divan::bench]
fn prepare_one_megabyte_of_paragraphs(bencher: divan::Bencher<'_, '_>) {
    bench_prepare(bencher, FIXTURE);
}

#[divan::bench]
fn prepare_one_megabyte_of_short_blocks(bencher: divan::Bencher<'_, '_>) {
    bench_prepare(bencher, SHORT_BLOCK);
}
