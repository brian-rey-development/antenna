# Antenna Roadmap

This file lists the work after the MVP. The MVP is in `docs/architecture.md` and `docs/plans/`. Each item here names its extension point in the architecture, so a later stage can start without a new design of the core.

An item moves into `docs/plans/` only when it has a stage file with acceptance criteria. The sequence in each horizon is the priority.

## Horizons

| Horizon | Meaning |
|---|---|
| Next | The first releases after v0.1.0. The design or the architecture already covers these items |
| Later | Items that need a new subsystem or a new design |
| Exploring | Ideas that need research before a decision |

## Next

### Studio

1. **Pause, emphasis and pronunciation marks.** The toolbar of the Studio design has these controls. A markup layer in `antenna-text` converts each mark into segment attributes. Each `EngineDescriptor` lists the attributes that the engine supports.
2. **Interpretation controls.** Stability, expressivity and speed, as in the inspector of the Studio design. Each `Variant` declares its interpretation settings and their ranges. `EngineFactory::load` receives the values.
3. **Synthesis while the user types.** When the user completes a sentence, a job synthesizes only that segment. The segment store already keeps all other segments.
4. **Word-level highlight.** `antenna-review` gives word timestamps from Whisper. The reading view highlights the current word in the signal color.
5. **Playback speed.** A time-stretch step in the player feed thread keeps the pitch at 0.75×, 1.25×, 1.5× and 2×.
6. **Version history.** The history button of the Studio design. Each save keeps the previous text. The segment store already keeps the audio of old versions until garbage collection.

### Library

1. **Background generation.** The Library generates the audio of a document while the user works in the Studio. `Pipeline::start` gets a `Priority { Studio, Library }` argument, and a Studio job goes in front of all Library jobs. When a Studio job arrives, the worker stops the running Library job after the current segment and puts it back in the queue. Its stored segments are cache hits when it runs again.

### Voices and languages

1. **More languages.** Qwen3-TTS also supports Japanese, Chinese, Korean and Russian. Magpie also supports Arabic, Hindi, Japanese, Korean, Chinese and Vietnamese. Each language needs a corpus for `antenna-eval`, WER limits and segmentation rules.
2. **More engines.** Kokoro-82M for low-memory machines, Kyutai Pocket TTS for CPU-only machines, Chatterbox Multilingual for emotion control. The procedure is in `docs/architecture.md` section 7.5.
3. **More designed voices.** More Qwen3 personas for each language from the VoiceDesign tool.

### Export

1. **Chapters.** Each Markdown heading becomes a chapter marker in MP3 (ID3 `CHAP` frames) and in Ogg (Vorbis comment chapters). `text::prepare` already keeps the source range of each segment. The export function of `antenna-audio` gets the heading positions from the timeline.
2. **Batch export.** Export a selection of documents from the Library in one action. The app runs one export after the other with the export function of `antenna-audio`. The job queue already runs jobs in sequence.
3. **More formats.** AAC in an M4A container through the macOS system encoder, and FLAC for archives. Each format is a new `ExportFormat` variant and one encoder module in `antenna-audio`.

### Platforms

1. **Linux and Windows packages.** CI already compiles and tests both. The work is packaging, signing and the update feed for each platform.

## Later

### Content

1. **Import of PDF, EPUB and web pages.** An `Importer` trait in `antenna-text` with one module for each format. Each importer keeps a map from the prose back to the source, for the highlight.
2. **Script rewrite with a local LLM.** An optional step before segmentation that rewrites a document as a podcast script. The user sees and edits the script before synthesis.
3. **Two-voice dialogues.** A script with two speakers, as in an interview. `Segment` gets an optional speaker, and `JobSpec` gets a map from speaker to voice.
4. **Pronunciation dictionary.** A list of words and their pronunciation for each user, applied by the markup layer.

### Voices

1. **Voice cloning from a short recording.** The user records or imports a clip and gets a new voice. This needs the speech tokenizer encoder and the speaker encoder in Rust, and a consent step in the UI.
2. **Voice design from a description.** The user writes a description, for example "warm, low, calm". Antenna makes a new voice from it with the Qwen3 VoiceDesign model.

### Distribution of episodes

1. **Podcast feed.** Antenna writes an RSS feed with the exported episodes of a Library folder. The user publishes the folder with any static host. The library already records the last export of each document. A new module in `antenna-library` writes the feed file.
2. **Intro and outro.** An audio clip at the start and the end of each export, with a crossfade. The extension point is a new step in the export function of `antenna-audio`, before loudness normalization.

### Integrations

1. **Local API.** A local HTTP server that accepts text and returns audio, for other apps on the same machine. It is a new app in `apps/` that uses the registry and the pipeline, as the CLI does.
2. **MCP server.** An MCP server with a `speak` tool and an `export` tool, so that AI assistants can use the local voices. It is a new app in `apps/` with the same composition root as the CLI.
3. **Watch folder.** The CLI watches a folder and exports each new document. It is a new `antenna watch` command that calls the export path of `antenna export`.

### Accessibility

1. **Screen reader support.** All screens work with VoiceOver. This needs a stable GPUI accessibility API and an ADR, as `docs/architecture.md` section 15 says.

## Exploring

1. **Studio quality mode with VoxCPM2.** It has 48 kHz output and voice design. Its speed on Apple Silicon has no measurement. A benchmark on an M5 Pro decides. The extension point is a new engine, `docs/architecture.md` section 7.5.
2. **iOS and iPadOS.** The engines run on Metal. The open questions are memory on phones and a second UI shell. The library crates stay the same, and the new shell is one more app in `apps/`.
3. **Translation.** A local translation model makes the same document in a second language. One Library entry keeps both versions. The extension point is a new step before `text::prepare`, at the same place as the LLM script rewrite.
4. **Expressive reading from document structure.** Use headings, quotes and lists to change the pace and the pauses without manual marks. The extension point is the markup layer of the pause and emphasis marks, which gets its attributes from the Markdown structure.
5. **Quantized weights.** Int8 or 4-bit talker weights to decrease memory and increase speed. A WER and listening comparison decides. The extension point is the `weights` module of `antenna-ml` and a new `Variant` of the engine.
