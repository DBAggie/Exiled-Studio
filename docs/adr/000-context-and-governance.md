# ADR-000: Context and Governance

- Status: Accepted (revised 2026-11-08)
- Date: 2026-02-01

## Context
The successor is a new application developed by an independent open-source group. The current Path of Building application is maintained by a different independent group. The successor must not assume control over the current project's releases, roadmap, deprecation, support policy, formats, or cooperation.

## Decision
The successor has independent governance, branding, releases, support policy, and repository boundaries. Compatibility with the current application is a cooperation-friendly objective, not an authority relationship. Public legacy formats, releases, and fixtures may be used where permitted, but no design depends on unpublished cooperation.

### Naming and license (2026-11-08)
- The product uses a distinct name that does not imply it is the next official version of Path of Building.
- **Working product name: Exiled Studio.** GitHub repository: `DBAggie/Exiled-Studio`, **public from day one**. The name gets a final trademark and name-collision check before the first public release.
- Crates and packages use the internal codename `loom` (e.g. `loom-domain`), so internals don't change if the public name does. "PathOfBuilding-V2" is a local folder name only.
- The README and the application's About screen state that the project is not affiliated with or endorsed by Grinding Gear Games or the Path of Building Community project.
- The successor is licensed under MIT, matching Path of Building, so code and findings can flow in both directions.

### Game intellectual property
Game files are used only to derive information that improves calculation and data accuracy. The successor ships derived data (stats, mods, bases, skills, tree structure). It does not ship or recreate GGG art, audio, or other game assets, except where GGG publishes them for tool developers.

## Consequences
- The successor owns its roadmap and contract versions.
- The current application remains an external compatibility target and regression oracle (see ADR-001).
- Documentation must avoid implying replacement, control, or shared maintenance.
- Contributions and bug findings may be shared upstream subject to each project's governance.
