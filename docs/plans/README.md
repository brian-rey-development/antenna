# Antenna Implementation Plan

The implementation has 19 stages. Each stage has one goal, a fixed scope and acceptance criteria that a command or a test can prove. An AI agent can do one stage from start to end without a human decision, unless the stage file says otherwise.

## 1. Stages

| Stage | File | Goal in short | Depends on |
|---|---|---|---|
| 01 | `01-foundation.md` | Workspace, tooling, CI, core contracts, fake engine, test kit, registry | None |
| 02 | `02-text.md` | Markdown to prose, segments, language identification | 01 |
| 03 | `03-audio.md` | Player with tracks, live output, export encoders, loudness, resampler | 01 |
| 04 | `04-models.md` | Model store, download, hash check, removal | 01 |
| 05 | `05-library.md` | Documents, segment store, search, garbage collection | 01 |
| 06 | `06-pipeline.md` | Job queue, job events, segment cache, engine pool | 02, 04, 05 |
| 07 | `07-cli.md` | The `antenna` command line app | 03, 06 |
| 08 | `08-ml-and-review.md` | Shared inference code and Whisper review | 03, 04 |
| 09 | `09-eval.md` | `antenna-eval` bench and WER, nightly CI | 06, 08 |
| 10 | `10-qwen3-decoder.md` | Qwen3 speech tokenizer decoder with parity | 08 |
| 11 | `11-qwen3-talker.md` | Qwen3 talker, code predictor and voice prompts with parity | 10 |
| 12 | `12-qwen3-engine.md` | Streaming `qwen3` engine that meets all budgets | 07, 09, 10, 11 |
| 13 | `13-magpie-model.md` | Magpie text frontend, encoder, decoder and local transformer with parity | 08 |
| 14 | `14-magpie-engine.md` | NanoCodec decoder and streaming `magpie` engine that meets all budgets | 07, 09, 12, 13 |
| 15 | `15-design-system.md` | `antenna-ui` with Flow tokens, fonts, light and dark themes, components | 01 |
| 16 | `16-desktop-studio.md` | App shell, Studio screen, player bar, export | 03, 06, 15 |
| 17 | `17-desktop-library.md` | Library, recent documents, Voices and Models screens | 08, 16 |
| 18 | `18-desktop-settings.md` | Settings, interface language, review, updates | 08, 17 |
| 19 | `19-release.md` | Signed macOS release v0.1.0 with updates | 12, 14, 18 |

### 1.1 Parallel work

1. After stage 01, stages 02, 03, 04, 05 and 15 can run at the same time.
2. After stage 08, the Qwen3 track (10, 11, 12) and stage 13 can run at the same time. Stage 14 starts after stage 12, because it adds its registration and its default voices after the `qwen3` lines.
3. Stages 16, 17 and 18 use the `fake` engine. Thus they can run at the same time as the engine tracks.

## 2. How to run a stage

All work goes to `main`. The remote has only the `main` branch.

1. If the stage runs alone, the agent works on `main` directly.
2. Stages that run at the same time (section 1.1) each get a git worktree. The agent makes it with the local branch `stage/<NN>-<name>` from `main`.
3. Give the agent the goal prompt in section 4.
4. The agent does the tasks in sequence and proves each acceptance criterion.
5. The agent does the quality gate in `docs/standards.md` section 20.
6. The agent writes the stage report `docs/reports/<NN>-<name>.md` in the last commit of the stage.
7. In a worktree, the agent rebases the branch onto `main`, runs `cargo xtask check` and fast-forwards `main`. Then it deletes the local branch.
8. The agent pushes `main`. CI runs on the push.

### 2.1 Stage report

The stage report is the record of the stage. It contains these items.

1. Each acceptance criterion with its evidence. The evidence is a test name, a command and its output, or a measurement.
2. The result of steps 2 and 3 of the quality gate in `docs/standards.md` section 20.
3. Each measurement of the stage.
4. Each difference from the plan that a decision rule permits, with the reason.
5. The results of the manual QA steps.
6. A sign-off line for each manual acceptance criterion. The project owner writes "approved" and the date on the line.

If the agent is blocked, it writes a "Blocked" section at the end of the stage file and stops. A human reads the section, makes the decision and adds it to the "Decision rules" of the stage.

## 3. Stage file format

Each stage file has these sections in this sequence.

| Section | Content |
|---|---|
| Goal | One sentence. The stage is complete when this sentence is true |
| Context | Why the stage exists and what it gives to the next stages |
| Read first | The documents and sections that the agent must read |
| Scope | The files and crates that the stage can create or change. All other files are read-only |
| Deliverables | The public API with Rust signatures, and the files |
| Tasks | Numbered steps in sequence. Each step is one instruction |
| Acceptance criteria | A table with an identifier, a criterion and the check that proves it |
| Decision rules | Answers to the problems that can occur |
| Out of scope | Work that the agent must not do in this stage |

A stage can add a section that only it needs, for example "Inputs from the project owner" or "Module benchmarks". An added section comes directly after the standard section that it extends. A stage can also add subsections with `###` in a standard section, for example "Facts to check" under "Decision rules".

Acceptance criteria have identifiers with the format `AC-<stage>-<number>`, for example `AC-03-07`. The check of a criterion is a test name, a command with its expected output, or a measurement with its limit. A criterion with the mark **(manual)** needs a human.

A command in a table cell is one command with no pipe. Write alternatives as `rg -e A -e B`. A pipe in the raw text of a table cell needs the escape `\|`. An agent that copies the raw command gets the escape and not a pipe. Put a check with a pipe or with more than one step in a numbered list under the table. The table cell names the list.

## 4. Goal prompt

Use this prompt for each stage. Replace `<NN>` and `<name>`.

```
Complete stage <NN> of Antenna as defined in docs/plans/<NN>-<name>.md.

Read CLAUDE.md, then every document in the "Read first" section of the stage file.
Do the tasks in sequence. Change only the files in the "Scope" section.
The stage is complete when:
1. every acceptance criterion has evidence,
2. `cargo xtask check` passes,
3. the quality gate in docs/standards.md section 20 is done,
4. the stage report docs/reports/<NN>-<name>.md is committed,
5. main contains the commits of the stage and is pushed.
Follow docs/plans/README.md section 2 for the worktree, the rebase and the push.
If you cannot meet a criterion and no decision rule applies, stop and write a "Blocked" section.
Do not change a test, lint, threshold, fixture or criterion to make a check pass.
```
