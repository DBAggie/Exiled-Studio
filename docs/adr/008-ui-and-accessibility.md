# ADR-008: UI, UX, and Accessibility

- Status: Accepted; Amendment 1 (passive tree editor before skills, proposed 2026-11-14, accepted on merge)
- Date: 2026-02-01

## Decision
Use a dual-mode progressive interface: guided task entry points plus a persistent dockable build workspace. The first workflow is import, dashboard, and a complete item editor; skills follow, then passive-tree editing.

Use React with a custom design system built on headless accessible primitives. The passive tree uses a hybrid Canvas 2D plus semantic DOM/SVG overlay renderer, with an independent accessible tree model. Standard UI targets WCAG 2.2 AA; custom visualizations provide equivalent keyboard and non-visual workflows.

Localization is English-first but localization-ready. Text is externalized, identifiers are language-neutral, and locale-aware formatting is required.

## Consequences
- Visual projections do not own domain state.
- Progressive disclosure must not change calculation meaning.
- Accessibility, keyboard navigation, scaling, contrast, reduced motion, and screen-reader workflows are release criteria.

## Amendment 1 (2026-11-14): The passive tree editor before skills

Decided by the maintainer on 2026-11-13 (after the Phase 4 PR #105: "implement the passive tree now", a full editor, order UI fidelity then the tree then Phase 5; after PR #106: "complete the passive tree and phase 5 in the next PR"). Proposed in the passive tree and Phase 5 PR (`dev/active/passive-tree-and-phase-5/`) and accepted when the maintainer merges it.

### Context
The decision above orders the workflows "import, dashboard, and a complete item editor; skills follow, then passive-tree editing", and the action plan put the tree editor in Phase 6+. After Phase 4 the maintainer judged the tree a core feature users expect from the first usable app, ahead of the skills screen. GGG's developer data page offers the passive tree export, with its sprite sheets, as the one exception to its rule against providing game data; ADR-000 allows GGG assets where GGG publishes them for tool developers.

### Decision
- **Order.** The workflows are import, dashboard, the item editor (Phase 4), then **passive-tree editing** as a full editor (allocation with path finding, deallocation, masteries, jewel sockets, ascendancy, cluster jewels, tree version switching), then skills. The action plan moves the tree editor out of Phase 6+ into the step before Phase 5.
- **Renderer unchanged; GGG's sprites with our shapes as fallback.** The hybrid Canvas 2D plus DOM/SVG overlay renderer and the independent accessible tree model stand. The canvas draws the export's sprite sheets when the data bundle carries them (ADR-005 Amendment 2; never committed to the repository), and our own vector shapes otherwise (tests, development bundles, or if the sprites are withdrawn). Node kinds stay distinguishable by shape and frame, not colour alone.
- **Non-visual parity.** Every tree action (allocate, deallocate, mastery, socket, class, ascendancy, version switch) is available in the non-visual tree view with the keyboard; the canvas is a projection that owns no domain state.

### Consequences
- The tree editor carries its own gate in the action plan (layout and allocation parity with GGG's export and PoB, cluster jewel fixtures, corpus, the app workflow with accessibility checks).
- The skills screen moves after the tree; nothing else in the order changes.
- The app shows GGG's third-party notice ("This product isn't affiliated with or endorsed by Grinding Gear Games in any way."), which GGG's developer docs ask of public applications.
