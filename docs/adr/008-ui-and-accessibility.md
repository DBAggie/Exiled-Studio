# ADR-008: UI, UX, and Accessibility

- Status: Accepted
- Date: 2026-02-01

## Decision
Use a dual-mode progressive interface: guided task entry points plus a persistent dockable build workspace. The first workflow is import, dashboard, and a complete item editor; skills follow, then passive-tree editing.

Use React with a custom design system built on headless accessible primitives. The passive tree uses a hybrid Canvas 2D plus semantic DOM/SVG overlay renderer, with an independent accessible tree model. Standard UI targets WCAG 2.2 AA; custom visualizations provide equivalent keyboard and non-visual workflows.

Localization is English-first but localization-ready. Text is externalized, identifiers are language-neutral, and locale-aware formatting is required.

## Consequences
- Visual projections do not own domain state.
- Progressive disclosure must not change calculation meaning.
- Accessibility, keyboard navigation, scaling, contrast, reduced motion, and screen-reader workflows are release criteria.
