---
name: exiled-studio-mcp
description: >
  How to use Exiled Studio's own MCP servers and read what they return. Use when a session has the
  app's MCP tools (get_build, list_fields, get_breakdown, preview_item_edit, get_tree, ...) and must
  answer about the open build, check a number, preview an edit, or explain why a field is missing;
  and when working on the MCP code, its contract, stat IDs, or divergences from PoB.
---

# Exiled Studio MCP (dev layer)

The knowledge pack's dev layer (plan `KNOWLEDGE-PACK-PLAN.md`, D5.2). It versions with the code: when a tool or rule here changes, change this file in the same PR.

## The two servers
| Server | Where | Transport | Status |
|---|---|---|---|
| App MCP (ADR-017) | `crates/loom-integrations/src/mcp`, inside the desktop app | Streamable HTTP on 127.0.0.1, bearer token (kept between starts, ADR-017 Amendment 1), off until the user starts it | Shipped; contract `schemas/mcp-tools.v1.json` |
| Knowledge MCP (ADR-018) | `tools/knowledge-mcp` | stdio, read-only, no network | **Proposed** (PR #123); not built. Do not assume its tools exist |

Connect a client to the app MCP once (the AI connection screen's **Copy command** gives it with the token):
```sh
claude mcp add --scope user --transport http exiled-studio <url> --header "Authorization: Bearer <token>"
```
Use `--scope user`: on Windows a project-scoped server is keyed by the folder's spelling (`D:/` vs `d:/`).

## App MCP tools
Read-only unless marked. Results are app API v1 DTOs (`schemas/app-api.v1.schema.json`).
| Tool | Use it for |
|---|---|
| `get_build` | Class, level, main skill, data version, calc status. Call first. |
| `list_fields` | Every calculated field with its value and **state** |
| `get_breakdown(field)` | One field's provenance graph: every node and contribution, each naming its source (item line, passive, gem, config) |
| `list_items`, `get_item(item)` | Items; each line's status (`matched`, `adapted`, `warning`, `excluded`) and the stat IDs it resolved to |
| `list_warnings` | Every line the app could not read: kept, shown, not calculated |
| `preview_item_edit(edit)` | Every field before and after an item edit, without applying it |
| `get_tree`, `find_tree_nodes(text)`, `get_tree_node(node)` | The passive spec and the loaded tree |
| `preview_tree_change(change)` | Every field before and after a tree change, without applying it |
| `request_item_edit`, `request_tree_change` (**write**) | Ask the user to apply a change; refused unless they allow requests (`APP_REQUESTS_OFF`); poll `get_request(id)` |

## Rules for answers
1. **Check every number before stating it.** DPS, EHP, resistances, the effect of a change: read it from `list_fields`/`get_breakdown`, or from `preview_item_edit`/`preview_tree_change` for a what-if. Quote the tool's value, not your own arithmetic. Without the app, label numbers **estimates**.
2. **States are not values.** `complete` is a number you can quote. `partial` is a number with something missing: say what (`get_breakdown`, `list_warnings`). `unsupported`, `indeterminate`, and `not-computed` (no value) are **never zero**: say the app does not compute it yet.
3. **Read `list_warnings` before a verdict.** An unread line is not in the numbers (ADR-015: unresolved content is kept and shown, never dropped).
4. **Never apply changes on your own.** Preview first; use a `request_*` tool only when the user asks, and tell them it waits for their approval in the app.
5. Edits applied through requests are ordinary undoable steps in the app's history.

## Concepts the results use
- **Stat IDs (ADR-015).** Every modifier is `(stat_id, values)`, the game's own stat IDs. Text is parsed only on import; a line's status says how it was read. `adapted` means an import adapter rewrote PoB-era text to the game's (ADR-015 Amendment 1).
- **The game is the truth, PoB the oracle (ADR-001, ADR-006).** Where Exiled Studio and Path of Building differ on purpose, the difference is a register entry in `docs/parity/divergence-register.json` (`DIV-nnnn`, kind `game-truth`, `pob-bug`, `data-version`, `harness`) with evidence. When a user says "PoB shows a different number", check the register before calling either one wrong. Example: a weapon's attack time is floored to whole milliseconds as the game does (Q11, DIV-0001 onward).
- **Data version.** A build is pinned to a data bundle (`<game>/<steam build>-s<schema>`, ADR-005, ADR-010); `get_build` names it. Numbers are for that game data, not necessarily the live patch.
- **Games.** PoE1 and PoE2 are separate data and rules. Establish which game before answering.

## Working on the MCP code
- Changing the tool list is a reviewed diff of the contract: `LOOM_WRITE_MCP_CONTRACT=1 cargo test -p loom-integrations --test mcp_contract`.
- Tool definitions are a public contract (ADR-017): additive changes only within v1.
- The knowledge MCP gets its own contract `schemas/knowledge-mcp-tools.v1.json` once ADR-018 is accepted; update this file then.
