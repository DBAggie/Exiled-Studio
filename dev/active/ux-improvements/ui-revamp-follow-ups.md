# Follow-up tasks after the UI revamp (2026-11-25)

Created at the maintainer's request ("create follow up tasks for each open item") when the stack #168 to #179 was merged into `dev`. One task an open item, from the revamp's steps (`ui-revamp-plan.md`, each step's "Not as the mockup") and from what was still open before it (`dev/active/open-items/maintainer-follow-ups.md`).

Each task is its own PR from `dev`. A task that changes how a screen looks shows a mockup for approval first. **Who:** "agent" can be done without the maintainer; "maintainer" needs a decision or an action that is the maintainer's.

## Revamp: left out of a step

| ID | Task | Why it is open | Who | Size |
|---|---|---|---|---|
| RV-01 | **Reset a dashboard to the default layout.** An action in Customize that puts the current default (four lead figures, Defence beside the Main skill) in place of a user's saved Overview, after asking. | Only the default changed in #175; a user with saved dashboards never sees the new layout. | agent, mockup first | small |
| RV-02 | **Bars for capped values in the Defence table** (block, resistances: the value against its cap). | Drawn on the Dashboard board, not built: the Defence panel's rows are generic. | agent | small |
| RV-03 | **A card under another in one column** (the Main skill over the lines to review, beside a tall Defence table). Needs a card to span rows, or a column component. | Drawn on the Dashboard board; the grid holds one card a cell. A layout field in build v1 `dashboards` (additive), so a decision first. | maintainer decides, then agent | medium |
| RV-04 | **The hover card's "points left after" line** on the passive tree. | Drawn on the tree board, not built. | agent | small |
| RV-05 | **"Discard all" for unsaved work** on the start page, with a confirmation. | Drawn on the Start board, not built. | agent | small |
| RV-06 | **The remaining pills that are not a calculation state:** an item line's status in the item editor, "Variant: …", the AI connection's pending count. Decide each: quiet text, the mark, or kept. | Step 3 replaced only the calculation state's pills. | agent, mockup first | small |
| RV-07 | **The item editor and the AI connection page** were not in the audit's six mockups: review them against the foundations and draw what should change. | Not covered by step 4. | agent, mockup first | medium |
| RV-08 | **A second pass with the audit list** (the `unslop` skill's signs and checklist) over the merged application, at 1,000, 1,280, and 1,600 px. | Each step was checked alone; nothing has checked the whole after all twelve PRs. | agent | small |
| RV-09 | **The tree's frame rate after the fade.** Measure pan and zoom on the test laptop as `docs/phase-5/performance.md` did (54/49 fps before). | #179 did not measure it. | maintainer runs, agent prepares | small |

## Revamp: needs work under the UI

| ID | Task | Why it is open | Who | Size |
|---|---|---|---|---|
| RV-10 | **Specific reasons on "Lines to review"** ("the game now says Charges"): the resolver names the nearest current line for a line it cannot read. | The mockups showed them; the UI shows the generic reason the resolver gives. `loom-serialization` resolver, ADR-015 territory. | agent, plan reviewed first | large |
| RV-11 | **A passive's uncounted modifier named by its line**, not by its passive ("Bone Barrier × 3"). | From the dashboard cards redesign (2026-11-25). | agent | medium |
| RV-12 | **Minion numbers for Raise Spectre**, so the Hit DPS card shows a value, not "Why". | A calculation gap (`dev/active/open-items/agent-items.md`), not a UI one. | agent | large |
| RV-13 | **An older application reading a newer recovery snapshot.** Test it; if the strict reader refuses the `summary` key, say so in `docs`, or make the reader of the next version lenient. | #173 did not test it. | agent | small |

## Decisions and actions that are the maintainer's

| ID | Task | Why it is open |
|---|---|---|
| RV-14 | **ADR-018's text still says the knowledge server ships "in the portable zip".** Amend it (an accepted ADR is not edited without the maintainer). | The portable zip was dropped on 2026-11-25. |
| RV-15 | **The PR test build:** keep CI's zip of the plain executable with its `portable` marker, or make the test build an installer and remove the marker code (`loom_tauri::release`, `tools/knowledge-mcp`). | The marker now serves only that zip. |
| RV-16 | **Run the release workflow once without the portable zip** (it runs on a tag or by hand from `main`) and check the download page lists four packages. | #168 changed `release.yml`; it could not be run from a branch. |
| RV-17 | **The macOS `rust` job's 15-minute limit:** it was cancelled at the limit on #171 and #173 and passed on a rerun in under 10 minutes. Raise the limit, or accept a rerun now and then. | Seen twice on 2026-11-25. |
| RV-18 | **A screen-reader pass** over the revamped screens (the marks' names, the folded lists, the two-pane Skills page). | The automatic checks do not replace it. |
| RV-19 | **The update test after the next experimental release**, and **ADR-015 Amendment 3** (proposed) to accept or change. | Open since before the revamp. |

## Open since before the revamp

| ID | Task | Who | Size |
|---|---|---|---|
| RV-20 | **Keyboard navigation on the tree's canvas** (move between linked nodes with the arrow keys). | agent, mockup first | medium |
| RV-21 | **Faster hover numbers on the tree** (a skill cache a calculation). | agent | medium |
| RV-22 | **Details as an inline panel**, not a dialog, on a dashboard card (the first mockup's form). | agent, mockup first | small |
| RV-23 | **The agent list** (`dev/active/open-items/agent-items.md`): Holy Relic Nova and the items after it. | agent | large |

## Added after the merge (2026-11-25)

| ID | Task | Who | Size |
|---|---|---|---|
| RV-24 | **Two new security advisories** (RUSTSEC-2026-0592, a stack overflow in `Url::origin()`; RUSTSEC-2026-0594, repeating cookie nonces) failed `cargo-deny advisories` on the bot's pull requests against `main` on 2026-11-25. Check `dev` and `main`, update the dependencies or record why each does not apply (`deny.toml`). | agent | small |
| RV-25 | **Repowise's optional parts:** its PR bot's AI review, runtime evidence uploads, and a line in `AGENTS.md` telling agents to ask Repowise first. Each is the maintainer's choice (the review spends credit). | maintainer decides | small |
| RV-26 | **The oracle fixture selection on a stacked pull request:** `oracle-select.py` fails (`git diff origin/<base>...HEAD` exits 128) when the base is another feature branch that the checkout has not fetched, as on #179. Fetch the base, or fall back to every fixture. | agent | small |

## Order suggested
RV-08 first (it may add tasks), then RV-01, RV-05, RV-04, RV-02 (small, visible), then RV-13, then the maintainer's decisions RV-14 to RV-17 at any time, then the larger ones.
