# UI design reference

Reference material for the UI work (ADR-008; the application slice is Phase 4 in `docs/plan/ACTION-PLAN.md`). Nothing here is built or shipped. Read it before starting UI work.

| File | What it is |
|---|---|
| `UI-DESIGN-BRIEF.md` | The design brief and decisions log (rough draft v1, 2026-11-11): product as understood, visual language, terminology, screen inventory, decisions, data-model implications, open questions. |
| `fidelity/` | The app's screens after the UI fidelity PR (2026-11-13), screenshot beside each mockup, with what changed and how the Phase 4 checklist's issues were fixed (`fidelity/README.md`). |
| `exiled-studio-ui-draft-v1.zip` | The draft package: its own `README.md` and a copy of the brief, full-page screenshots of the 11 screens and extra states, self-contained rendered HTML, the editable design-canvas source (`canvas-source/project/*.dc.html`), the Python scripts that built and rendered them, and a 74 s walkthrough video (out of date, see the package README). |

All numbers, items, and builds in the mockups are sample data for layout only. Item and gem pictures are placeholder drawings by slot type and simple shapes; no game art is used (ADR-000; checked when the package was added: no image references in the HTML other than its own script, screenshots show placeholder drawings only).

The brief names a live design canvas private to the maintainer; if it was edited after 2026-11-11, it is newer than this package.
