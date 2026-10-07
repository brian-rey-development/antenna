use std::sync::LazyLock;

use antenna_core::{Document, Language};
use whatlang::{Detector, Info, Lang};

use crate::prose::Prose;

const IDENTIFY_SAMPLE_CHARS: usize = 2_000;
const IDENTIFY_MIN_LETTERS: usize = 20;

static DETECTOR: LazyLock<Detector> =
    LazyLock::new(|| Detector::with_allowlist(Language::ALL.map(lang_of).to_vec()));

fn lang_of(language: Language) -> Lang {
    match language {
        Language::En => Lang::Eng,
        Language::Es => Lang::Spa,
        Language::Pt => Lang::Por,
        Language::Fr => Lang::Fra,
        Language::It => Lang::Ita,
        Language::De => Lang::Deu,
    }
}

/// Identifies the language of a document from the first part of its prose. The function returns
/// `None` if the sample has too few letters or the result is not reliable. It does not use the
/// locale of the system.
pub fn identify_language(document: &Document) -> Option<Language> {
    let prose = Prose::for_document(document);
    let sample: String = prose.text().chars().take(IDENTIFY_SAMPLE_CHARS).collect();
    let letters = sample.chars().filter(|character| character.is_alphabetic());
    if letters.count() < IDENTIFY_MIN_LETTERS {
        return None;
    }
    let lang = DETECTOR.detect(&sample).filter(Info::is_reliable)?.lang();
    Language::ALL
        .into_iter()
        .find(|language| lang_of(*language) == lang)
}

#[cfg(test)]
mod tests {
    use antenna_core::TextFormat;

    use super::*;

    #[test]
    fn identify_language_ignores_code_when_markdown_has_code_block() {
        let code = format!(
            "```\n{}```\n\n",
            "the quick brown fox jumps over the lazy dog\n".repeat(60)
        );
        let spanish = "El viejo guardián subió la larga escalera de caracol y contó cada peldaño. ";
        let text = format!("{code}{}", spanish.repeat(3));
        let document = Document::new(text, TextFormat::Markdown).unwrap();

        assert_eq!(identify_language(&document), Some(Language::Es));
    }

    #[test]
    fn identify_language_returns_none_when_text_has_no_letters() {
        let document =
            Document::new("12 34 56 78 90 12 34 56 78 90 12 34", TextFormat::Plain).unwrap();

        assert_eq!(identify_language(&document), None);
    }
}
