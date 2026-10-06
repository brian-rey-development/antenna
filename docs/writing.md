# Antenna Writing Standard (STE-80)

This standard applies to all English text in the repository. That includes documents, doc comments, error messages, log messages, commit messages, stage reports and UI copy.

STE-80 is a profile of ASD-STE100 Issue 9 (January 2025). It keeps the rules that make text clear for human readers and for AI agents. It relaxes the rules that do not fit software documents. The full standard is free at [asd-ste100.org](https://www.asd-ste100.org/).

## 1. Rules

### 1.1 Sentences

1. Write a maximum of 20 words in an instruction sentence.
2. Write a maximum of 25 words in a descriptive sentence.
3. Write one instruction in each sentence. Two actions can be in one sentence only if they occur at the same time.
4. Write a maximum of 6 sentences in a paragraph. Each paragraph has one topic.
5. Do not omit articles (`a`, `an`, `the`). Do not use contractions.
6. Do not use semicolons.
7. Use a vertical list for a sequence of steps. Use a vertical list for more than three items when each item needs a description. A list of more than three names, for example types, files or crates, can stay in a sentence.
8. A code identifier, a command, a path or a number with its unit counts as one word.

### 1.2 Verbs

1. Use the imperative in instructions. Write "Run the command." Do not write "You must run the command."
2. Put a condition first, then a comma, then the instruction. Example, "If the hash is different, delete the file."
3. Use the active voice and name the agent. Example, "The worker sends the chunk to the sink."
4. Use only the simple present, the simple past and the simple future.
5. Use only the modal verbs `can`, `must` and `will`. Use `must not` and `do not` for prohibitions.
6. Do not use `should`, `may`, `might`, `would` or `could`. A reader cannot know if `should` states a requirement. Write `must` for a requirement and `can` for a possibility.
7. Use an `-ing` word only in a technical noun, for example "the streaming playback".
8. Use a verb for an action. Write "The engine loads the weights", not "The engine does the loading of the weights".

### 1.3 Words

1. Use one term for one concept. The glossary in section 4 is the reference. Use the glossary term in code, documents and messages.
2. Write a maximum of three nouns in a sequence. Write "the play position of the track", not "the track play position counter value".
3. Use technical nouns and technical verbs from software and audio engineering. Examples are crate, thread, buffer, chunk, compile, deploy, commit, merge, download and load.
4. Use a technical noun only as a noun. Write "Put the samples in the buffer", not "Buffer the samples".
5. Use the approved word from section 2 instead of an unapproved word.
6. Use American spelling.

### 1.4 Notes and warnings

1. A note gives information only. Do not put an instruction or a limit in a note.
2. Use `WARNING` for a risk of data loss. Use `CAUTION` for a risk of a defect or a failed build.
3. Start a warning or a caution with a command, then give the reason.

### 1.5 Relaxed rules

These STE rules do not apply in this repository.

1. The full dictionary check. Section 2 replaces it.
2. Word counts in tables, headings and code blocks.
3. The technical-noun rules for code identifiers in backticks.
4. Established software verbs with two words, for example `log in`, `roll back`, `check out` and `set up`.
5. `check` and `test` as verbs, because they are the names of commands in this project.

## 2. Word substitutions

Use the word in the right column.

| Do not use | Use |
|---|---|
| ensure, verify, confirm | make sure |
| utilize, leverage | use |
| perform, carry out, accomplish | do |
| prior to | before |
| within | in |
| certain | some |
| commence, initiate, begin | start |
| terminate | stop |
| obtain, acquire | get |
| require | need, is necessary |
| additional | more |
| indicate | show |
| retain | keep |
| initial | first |
| modify | change |
| allow | let |
| detect | find |
| main | primary |
| e.g., i.e., etc. | for example, that is, and so on |
| should | must, or can |
| may, might | can |
| robust, seamless, powerful, comprehensive | the measured property: "restarts in 200 ms" |
| simply, just, easily | (delete the word) |

## 3. Patterns that are prohibited

These patterns make text longer and less precise. Text with these patterns fails review.

| Pattern | Example of bad text | Correct text |
|---|---|---|
| Negative contrast | "This is not a wrapper, it is an engine." | "This crate is an engine." |
| Groups of three for rhythm | "Fast, clean and reliable." | "Synthesis runs at 0.3 RTF." |
| Inflated adjectives | "A powerful, robust pipeline" | "The pipeline cancels a job in one chunk." |
| Summary at the end | "In summary, this section explained..." | (delete the summary) |
| Questions as headings | "Why threads?" | "Threads for synthesis" |
| Promises without numbers | "Very fast startup" | "The app is interactive in 300 ms." |
| Hedge words | "This can possibly help to improve..." | "This change decreases the latency by 40 ms." |
| Em dash or en dash | "The engine (fast) loads..." with dashes | Use a full stop, a comma or parentheses |
| Emoji | Any emoji | (delete it) |

## 4. Glossary

One concept has one term. Use these terms exactly.

| Term | Definition | Code name |
|---|---|---|
| document | The complete text that the user gives to Antenna | `Document` |
| source range | A byte range in the document | `Range<usize>` |
| segment | A part of the document that an engine synthesizes in one call. It is one sentence, or one clause of a long sentence | `Segment` |
| sample | One `f32` value of mono PCM audio | `f32` |
| chunk | A sequence of samples that an engine emits in one call of the emit callback | `PcmChunk` |
| sample rate | The number of samples in one second of audio | `SampleRate` |
| engine | An object that converts segments into chunks with one loaded voice | `Engine` |
| engine factory | An object that describes an engine type and loads engines | `EngineFactory` |
| engine descriptor | The static data that tells the capabilities of an engine type | `EngineDescriptor` |
| voice | One speaker identity of an engine for one language | `VoiceDescriptor`, `VoiceId` |
| default voice | The voice that Antenna selects for a language when the user did not select one | `Registry::default_voice` |
| registry | The list of all engine factories in the build, with the default voices | `Registry` |
| artifact | One model file that Antenna downloads | `Artifact` |
| model files | The local paths of the artifacts of one voice | `ModelFiles` |
| model store | The local directory that contains the downloaded artifacts | `ModelStore` |
| job | One run of the pipeline over one document with one voice | `JobSpec`, `JobHandle` |
| output | An object that the pipeline opens with the engine sample rate to get a sink | `Output` |
| sink | An object that receives the samples of a job while the job runs | `Sink` |
| playback | The output of samples to the audio device | `Player`, `LiveOutput` |
| track | The list of the segments of a document that the player plays | `Track` |
| play position | The segment and the time in the segment that the player plays now | `TrackPosition` |
| export | The conversion of the stored segments of a document into an MP3, WAV or Ogg Opus file | `antenna_audio::export`, `ExportFormat` |
| segment index | The position of a segment in its document | `SegmentIndex` |
| segment store | The directory that keeps the audio of each synthesized segment | `SegmentStore` |
| segment key | The SHA-256 that names a stored segment | `SegmentKey` |
| timeline | The segments, keys and stored durations of a document for one voice and quality | `Timeline` |
| text hash | The SHA-256 of the format and the text of a document | `TextHash` |
| engine pool | The loaded engines that the job worker keeps for the next jobs | `EnginePool` |
| variant | The files and the model size of an engine for one quality | `Variant`, `Variants` |
| synthesis front | The end of the generated audio in the document while a job runs | (none) |
| job event | A message from a job to the UI | `JobEvent` |
| composition root | The code that creates the concrete objects and connects them | `antenna-engine-registry`, the `main.rs` of each app and of `antenna-eval` |
| conformance suite | The tests that each engine must pass | `antenna-engine-testkit` |
| reference implementation | The official implementation of a model. For both MVP engines, it is in Python | (none) |
| parity test | A test that compares the output of an in-house engine with fixtures from the reference implementation | `src/<module>/parity.rs` |
| time to first audio (TTFA) | The time from the start of a job to the first sample at the sink | `ttfa_warm`, `ttfa_cold` |
| real-time factor (RTF) | Synthesis time divided by audio duration | `rtf` |
| stage | One unit of planned work with one goal | `docs/plans/NN-*.md` |
| acceptance criterion | A condition that a stage must satisfy, with a check that proves it | (none) |
