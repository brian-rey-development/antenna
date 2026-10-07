use antenna_core::{Document, Language, TextFormat};
use antenna_text::identify_language;

use crate::support::{FIXTURES, fixture};

#[test]
fn identify_language_finds_language_when_fixture() {
    for (language, text) in FIXTURES {
        let document = Document::new(text, TextFormat::Markdown).unwrap();

        assert_eq!(identify_language(&document), Some(language));
    }
}

#[test]
fn identify_language_reads_only_sample_when_text_is_long() {
    let english = fixture(Language::En);
    let sample = english.repeat(2_000 / english.len() + 1);
    let spanish = fixture(Language::Es).repeat(2 * sample.len() / fixture(Language::Es).len());
    let document = Document::new(format!("{sample}{spanish}"), TextFormat::Plain).unwrap();

    assert_eq!(identify_language(&document), Some(Language::En));
}

#[test]
fn identify_language_returns_none_when_text_is_short() {
    let document = Document::new("Hello there, my friend.", TextFormat::Plain).unwrap();

    assert_eq!(identify_language(&document), None);
}

#[test]
fn identify_language_returns_none_when_unreliable() {
    let mixed = "Hello world. Hola mundo. Bonjour le monde. Ciao mondo. Hallo Welt. Ola mundo.";
    let document = Document::new(mixed, TextFormat::Plain).unwrap();

    assert_eq!(identify_language(&document), None);
}
