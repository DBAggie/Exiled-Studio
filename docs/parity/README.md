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
| `fixture` | The fixture: its id (the file name, as `oracle-run` reports it) or `sha256:<hex>` of its bytes on either side (for corpus builds, which are named by hash). Name each fixture one way only: the duplicate check compares the text, so an id entry and a hash entry for the same fixture and field would both pass it, and the first one found would apply. |
| `field` | A JSON pointer (RFC 6901) into the oracle output document, under `/stage`, `/partial`, `/main_output`, or `/breakdowns`. Example: `/main_output/MovementSpeedMod`, `/breakdowns/Life/2`. |
| `comparison` | `{"type": "exact"}`: the two values must be exactly `oracle_value` and `successor_value`. `{"type": "abs", "tolerance": t}`: numbers `a`, `b` are within it when `abs(a - b) <= t`. `{"type": "rel", "tolerance": t}`: when `abs(a - b) <= t * max(abs(a), abs(b))`. A tolerance entry covers a difference when left and right are within it of each other, left of `oracle_value`, and right of `successor_value`; so a value that moves is unexpected again, even if the two sides stay close. |
| `oracle_value`, `successor_value` | The values when the entry was added; `null` means the field is absent on that side (output trees never hold `null`, so this is unambiguous). For `abs` and `rel`, both are numbers within the tolerance of each other. |
| `kind` | `game-truth` (the game behaves as the successor computes; game evidence required, ADR-001), `pob-bug`, `data-version` (game data differs between the oracle's tag and the successor), `harness` (the harness causes it). |
| `evidence` | Links or citations (at least one). |
| `test` | The test that pins the difference. |
| `added` | Date, `YYYY-MM-DD`. |
| `status` | `accepted` entries cover differences; `retired` ones are kept for history and cover nothing. |

Rules beyond the schema: ids are unique; at most one `accepted` entry per fixture and field; an `exact` entry's two values differ; a tolerance entry's two values are within its tolerance. Numbers compare as doubles, so `1` equals `1.0` (and integers above 2^53 lose precision, which PoB's doubles cannot produce anyway).

## How differences are classified

`loom-parity-cli oracle-compare <left> <right> --out <dir> [--register <file>]` compares two output documents of `oracle-run` (`outputs/<id>.json`), or every fixture of two `oracle-run` output directories. The left side is the oracle; the right side is the candidate (the successor in Phase 3; another platform, run, or fixture today).

- Compared: `stage`, `partial`, `main_output`, and the selected `breakdowns`, leaf by leaf. Numbers compare as doubles, exactly. The other fields of the document record the run (modules, settings probe, shims, probe) and are listed in the report as not compared, with the reason.
- A difference covered by an `accepted` entry for its fixture and field is `ORACLE_EXPECTED_DIFFERENCE`. Any other is `ORACLE_UNEXPECTED_DIFFERENCE`; when an entry exists for the field but does not cover the values, the report says why.
- Output documents must be complete (exactly the protocol's fields, stage `done`); a truncated one is refused, never compared as empty.
- A fixture that has no output on a side (it failed there) is not compared and keeps the failing side's class: an environment or calculation failure is never read as a difference, nor as a pass. The exception: both sides failed with the class their fixture manifest expects (`malformed.xml`), which is an `expected_failure` match.
- In two run directories, a fixture whose bytes differ between the sides is flagged `input_differs` (expected for PoB's own fixtures across platforms: CRLF on Windows, LF on Linux).
- Accepted entries for a compared fixture that covered nothing are listed (`unused_entries`): the difference is gone, and the entry may be retired.

Exit codes: a failure class of a run (10 to 18) when a fixture has no output on a side; else 20 when any difference is unregistered or a fixture is on one side only; else 19 when every difference is registered; else 0. One unregistered difference makes the whole comparison 20, whatever else is expected. A gate passes on 0 and 19.

## Adding an entry

1. Reproduce the difference with `oracle-compare`, and find out which side is right: game evidence for `game-truth` (ADR-001), a PoB issue or code citation for `pob-bug`. GGG's official patch notes are a primary source for `game-truth` and `data-version` entries (a mechanic or value changed in a patch after the oracle's tag): PoE1 [Patch Notes forum](https://www.pathofexile.com/forum/view-forum/patch-notes), PoE2 [Early Access Patch Notes forum](https://www.pathofexile.com/forum/view-forum/2212); cite the thread URL in `evidence`.
2. Add a test that pins the successor's value.
3. Add the entry with the next free id, `exact` unless the difference is numerical noise that has a written reason for its tolerance.
4. Run `loom-parity-cli divergence-check docs/parity/divergence-register.json` (also part of `cargo test`), and `oracle-compare` with `--register` to see it classified as expected.

The intentional-discrepancy fixture pair (`tools/oracle/fixtures/minimal-level1.xml` and `discrepancy-movement-speed.xml`) and its test register (`tools/oracle/fixtures/discrepancy-register.json`) show the whole path: unexpected without the register, expected with it. The test register is not part of this register.

## Where the oracle side comes from

`tools/oracle/` holds the pinned oracle (manifest, adapter, fixtures); `AGENTS.md` lists the commands; `dev/completed/spike-c-oracle-harness/` holds the plan and the measured facts (Spike C).

### Where the oracle chooses for PoB
Shims normally only undo headless artifacts (`inflate`, `write_guard`). One shim chooses a result that PoB itself leaves to chance: `variant_order` (adapter protocol 3, PR C3b+C4). PoB v2.67.2 loads a Vaal gem saved without a `variantId` (the save format before 3.23) as whichever of three variants (base, `AltX`, `AltY`) `pairs` yields first, which changes from run to run. The oracle always loads the **base** variant, the gem such a save meant; each output lists the affected gems in `probe.variant_order`. A successor that loads such a gem differently from the oracle needs a register entry like any other difference.
