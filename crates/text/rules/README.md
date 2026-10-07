# Sentence rules

The file `segment.srx` contains the sentence segmentation rules of LanguageTool.
The `srx` crate reads the file.

| Item | Value |
|---|---|
| Attribution | Segmentation rules from LanguageTool |
| Source | https://github.com/languagetool-org/languagetool/blob/be43b493a6155e612a30b1aaadb84ebfd42c92cf/languagetool-core/src/main/resources/org/languagetool/resource/segment.srx |
| Commit | `be43b493a6155e612a30b1aaadb84ebfd42c92cf` (2026-09-10) |
| License | LGPL-2.1-or-later |

The file is a copy of the file at the commit with one change. The repository forbids the characters U+2013 and U+2014 in text files, so the copy writes them as the XML references `&#x2013;` and `&#x2014;`. An XML parser reads the same characters.

The code in `src/sentences.rs` also changes the rules in memory before the parse. The `srx` crate reads the break position from the first capture group of a rule, so the code turns each capture group of a `beforebreak` pattern into a non-capturing group.

To update the file, copy it from a newer commit, write the references again and write the new commit here.
