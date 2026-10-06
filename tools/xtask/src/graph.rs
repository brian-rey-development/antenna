//! The permitted dependency graph of `docs/architecture.md` section 3.1, as data.

/// The permitted workspace dependencies of one crate. Each path is relative to the workspace root.
#[derive(Debug)]
pub(crate) struct Row {
    pub(crate) path: &'static str,
    pub(crate) normal: &'static [&'static str],
    pub(crate) dev: &'static [&'static str],
}

const CORE: &str = "crates/core";
const UI: &str = "crates/ui";
const TEXT: &str = "crates/text";
const AUDIO: &str = "crates/audio";
const PIPELINE: &str = "crates/pipeline";
const MODELS: &str = "crates/storage/models";
const LIBRARY: &str = "crates/storage/library";
const ML: &str = "crates/inference/ml";
const REVIEW: &str = "crates/inference/review";
const REGISTRY: &str = "crates/engines/registry";
const TESTKIT: &str = "crates/engines/testkit";
const FAKE: &str = "crates/engines/fake";
const QWEN3: &str = "crates/engines/qwen3";
const MAGPIE: &str = "crates/engines/magpie";

const NONE: &[&str] = &[];
const ONLY_CORE: &[&str] = &[CORE];
const ENGINE: &[&str] = &[ML, CORE];
const ENGINE_DEV: &[&str] = &[TESTKIT, MODELS];

/// The rows of the table, one for each crate of the workspace and of the later stages.
pub(crate) const GRAPH: [Row; 18] = [
    Row {
        path: CORE,
        normal: NONE,
        dev: NONE,
    },
    Row {
        path: UI,
        normal: NONE,
        dev: NONE,
    },
    Row {
        path: ML,
        normal: NONE,
        dev: NONE,
    },
    Row {
        path: TEXT,
        normal: ONLY_CORE,
        dev: NONE,
    },
    Row {
        path: AUDIO,
        normal: ONLY_CORE,
        dev: NONE,
    },
    Row {
        path: MODELS,
        normal: ONLY_CORE,
        dev: NONE,
    },
    Row {
        path: LIBRARY,
        normal: ONLY_CORE,
        dev: NONE,
    },
    Row {
        path: TESTKIT,
        normal: ONLY_CORE,
        dev: NONE,
    },
    Row {
        path: REVIEW,
        normal: &[ML, AUDIO, CORE],
        dev: &[MODELS],
    },
    Row {
        path: FAKE,
        normal: ONLY_CORE,
        dev: &[TESTKIT],
    },
    Row {
        path: QWEN3,
        normal: ENGINE,
        dev: ENGINE_DEV,
    },
    Row {
        path: MAGPIE,
        normal: ENGINE,
        dev: ENGINE_DEV,
    },
    Row {
        path: REGISTRY,
        normal: &[FAKE, QWEN3, MAGPIE, CORE],
        dev: NONE,
    },
    Row {
        path: PIPELINE,
        normal: &[TEXT, MODELS, LIBRARY, CORE],
        dev: &[TESTKIT, FAKE],
    },
    Row {
        path: "apps/cli",
        normal: &[REGISTRY, PIPELINE, TEXT, MODELS, LIBRARY, AUDIO, CORE],
        dev: NONE,
    },
    Row {
        path: "apps/desktop",
        normal: &[
            UI, REGISTRY, PIPELINE, TEXT, MODELS, LIBRARY, AUDIO, REVIEW, CORE,
        ],
        dev: NONE,
    },
    Row {
        path: "tools/eval",
        normal: &[REGISTRY, PIPELINE, MODELS, LIBRARY, REVIEW, CORE],
        dev: NONE,
    },
    Row {
        path: "tools/xtask",
        normal: NONE,
        dev: NONE,
    },
];
