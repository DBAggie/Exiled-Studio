# Parity: the divergence register

The game is the truth; Path of Building (PoB) at a pinned tag is the regression oracle (ADR-001, ADR-006). When the successor's result differs from the oracle's, the difference is either a bug in the successor or a recorded, evidenced divergence. This folder holds that record.

| File | What it is |
|---|---|
| `divergence-register.json` | The register: every accepted difference. Empty until Phase 3 compares a successor result. |
| `divergence-register.schema.json` | The format as JSON Schema (documentation). `loom-parity-cli divergence-check` enforces it, plus the rules a schema cannot express. |

Unregistered differences fail a parity gate. There is no general "close enough" rule: every tolerance is written down per field, in an entry (ADR-006, "Comparison rules").

## Entry fields

| Field | Meaning |
|---|---|
| `id` | `DIV-` and four digits. Unique; never reused, even after an entry is retired. |
| `mechanic` | The game mechanic the difference is about. |
| `fixture` | The fixture: its id (the file name, as `oracle-run` reports it) or `sha256:<hex>` of its bytes (for corpus builds, which are named by hash). |
| `field` | A JSON pointer (RFC 6901) into the oracle output document, under `/stage`, `/partial`, `/main_output`, or `/breakdowns`. Example: `/main_output/MovementSpeedMod`, `/breakdowns/Life/2`. |
| `comparison` | `{"type": "exact"}`: the two values must be exactly `oracle_value` and `successor_value`. `{"type": "abs", "tolerance": t}`: numbers with `abs(left - right) <= t`. `{"type": "rel", "tolerance": t}`: numbers with `abs(left - right) <= t * max(abs(left), abs(right))`. |
| `oracle_value`, `successor_value` | The values when the entry was added; `null` means the field is absent on that side. For `exact`, they are the values the entry covers, so a changed value is unexpected again. For `abs` and `rel`, they are a record; the tolerance decides. |
| `kind` | `game-truth` (the game behaves as the successor computes; game evidence required, ADR-001), `pob-bug`, `data-version` (game data differs between the oracle's tag and the successor), `harness` (the harness causes it). |
| `evidence` | Links or citations (at least one). |
| `test` | The test that pins the difference. |
| `added` | Date, `YYYY-MM-DD`. |
| `status` | `accepted` entries cover differences; `retired` ones are kept for history and cover nothing. |

Rules beyond the schema: ids are unique; at most one `accepted` entry per fixture and field; an `exact` entry's two values differ (numbers compare as doubles, so `1` equals `1.0`).

## How differences are classified

`loom-parity-cli oracle-compare <left> <right> --out <dir> [--register <file>]` compares two output documents of `oracle-run` (`outputs/<id>.json`), or every fixture of two `oracle-run` output directories. The left side is the oracle; the right side is the candidate (the successor in Phase 3; another platform, run, or fixture today).

- Compared: `stage`, `partial`, `main_output`, and the selected `breakdowns`, leaf by leaf. Numbers compare as doubles, exactly. The other fields of the document record the run (modules, settings probe, shims, probe) and are listed in the report as not compared, with the reason.
- A difference covered by an `accepted` entry for its fixture and field is `ORACLE_EXPECTED_DIFFERENCE`. Any other is `ORACLE_UNEXPECTED_DIFFERENCE`; when an entry exists for the field but does not cover the values, the report says why.
- A fixture that has no output on a side (it failed there) is not compared and keeps that side's class: an environment or calculation failure is never read as a difference, nor as a pass.
- Accepted entries for a compared fixture that covered nothing are listed (`unused_entries`): the difference is gone, and the entry may be retired.

Exit codes: 0 identical; 19 only registered differences; 20 an unregistered difference (or a fixture present on one side only); a smaller failure class (10 to 18) when a fixture has no output on a side. A gate passes on 0 and 19.

## Adding an entry

1. Reproduce the difference with `oracle-compare`, and find out which side is right: game evidence for `game-truth` (ADR-001), a PoB issue or code citation for `pob-bug`.
2. Add a test that pins the successor's value.
3. Add the entry with the next free id, `exact` unless the difference is numerical noise that has a written reason for its tolerance.
4. Run `loom-parity-cli divergence-check docs/parity/divergence-register.json` (also part of `cargo test`), and `oracle-compare` with `--register` to see it classified as expected.

The intentional-discrepancy fixture pair (`tools/oracle/fixtures/minimal-level1.xml` and `discrepancy-movement-speed.xml`) and its test register (`tools/oracle/fixtures/discrepancy-register.json`) show the whole path: unexpected without the register, expected with it. The test register is not part of this register.

## Where the oracle side comes from

`tools/oracle/` holds the pinned oracle (manifest, adapter, fixtures); `AGENTS.md` lists the commands; `dev/active/spike-c-oracle-harness/` holds the plan and the measured facts (Spike C).
