# Glossary and tone

Two files govern the words in a Builder tool. `GLOSSARY.md` says what each term means. `TONE.md` says how every string is written. Both are read by the agent before it names anything, and both are checked by `crates/core/tests/copy.rs`.

## Why a glossary

A tool built over several sessions gets described by several sessions, and each one reaches for slightly different words. A player meets a "spout" on one screen and a "nozzle" on the next, and has no way to know they are the same thing. In a classroom tool the cost is higher: a term that drifts is a term students learn wrong.

The glossary is the source of truth. When a string, a test name, a rule or the plan uses a term, it uses it in the glossary's sense. The practice comes from Peter's Mandible, which keeps 75 terms with one-sentence definitions and shows each definition when a player hovers the word. The `Not:` line comes from Slushline, whose copy test caught the agent's own settings strings calling a spout a "nozzle" and a "tap".

For a person who knows a field and does not write code, the glossary is also where their expertise enters the build in its most exact form. The terms of their field, with the near-synonyms that mean something else, go into the glossary first, and the tests then hold every later string to them.

## The format

```text
### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
```

- A `### Term` heading per entry, in alphabetical order.
- One or two plain sentences saying what the term means.
- A number a rule decides is a `{placeholder}`. Mandible's definitions repeat rule numbers ("Brood Cell: Adds 8 Housing"); when a balance change moves the number in the core, a typed number goes stale and nothing notices. A placeholder is filled by the core, so the two cannot disagree.
- An optional `Not:` line: words that must never be used for this term.
- Text inside an HTML comment does not count.

The core parses the file (`crates/core/src/glossary.rs`), so the page's hover hints and the tests read it the same way. `scripts/build.sh` copies it to `dist/data/glossary.md` for the page.

## What the tests check

| Test | Fails when |
| --- | --- |
| `glossary.rs` unit tests | an entry has no definition, or a term is defined twice |
| `every_glossary_entry_is_complete_and_states_no_bare_number` | a definition types a digit outside a `{placeholder}` |
| `no_reader_string_uses_a_word_the_glossary_rules_out` | a string in `data/` uses a word from a `Not:` line |
| `no_text_uses_a_phrase_the_tone_file_rules_out` | a string in `data/`, a definition or a line of `README.md` uses a phrase listed in `TONE.md` |
| `the_tone_lint_catches_each_listed_phrase` | the lint would miss one of its own phrases |

A word that is right in one file goes in `ALLOWED` in `copy.rs` with its reason, so the exception is visible in review.

## Tone

`TONE.md` holds the rules every reader-visible string follows: lead with the point, plain words and short sentences, one name for each thing, numbers from the core, refusals that name what is in the way and the next step, no universals the tool cannot back, colour never the only cue, no hype. The phrases between its lint markers are the ones a test can check: stock words, announcements of a point, contrast formulas, filler and exclamation marks. A person adds their own.

A test reads text, not meaning. A list of three written for rhythm, a slogan, or a claim the tool cannot back passes it; those are for the person's reading. The agent lists every string it wrote at the foot of `SECOND-ORDER.md`, and a string that looks wrong is reported, not repaired, because the wording may be the person's.
