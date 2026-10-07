use antenna_core::Language;

use crate::support::{MARKDOWN, fixture, snapshot};

#[test]
fn prepare_snapshot_matches_when_english_fixture() {
    insta::assert_snapshot!(snapshot(fixture(Language::En), Language::En));
}

#[test]
fn prepare_snapshot_matches_when_spanish_fixture() {
    insta::assert_snapshot!(snapshot(fixture(Language::Es), Language::Es));
}

#[test]
fn prepare_snapshot_matches_when_portuguese_fixture() {
    insta::assert_snapshot!(snapshot(fixture(Language::Pt), Language::Pt));
}

#[test]
fn prepare_snapshot_matches_when_french_fixture() {
    insta::assert_snapshot!(snapshot(fixture(Language::Fr), Language::Fr));
}

#[test]
fn prepare_snapshot_matches_when_italian_fixture() {
    insta::assert_snapshot!(snapshot(fixture(Language::It), Language::It));
}

#[test]
fn prepare_snapshot_matches_when_german_fixture() {
    insta::assert_snapshot!(snapshot(fixture(Language::De), Language::De));
}

#[test]
fn prepare_snapshot_matches_when_markdown_fixture() {
    insta::assert_snapshot!(snapshot(MARKDOWN, Language::En));
}
