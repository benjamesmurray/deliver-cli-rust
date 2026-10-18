# remove-mcpx-references - Specification

Per temp/CR_2026-10-18_remove_mcpx_references_from_hints_and_shims.md: reword mcpx hints in spec_manager.rs, remove dead shims, add archive hint on last sc_todo_complete.

## Core Features & Scope
### In-Scope
- Reword the 12 `mcpx` hint/error strings in `src/manager/spec_manager.rs` to plain "call <tool>" wording.
- Update `.antigravity.md` mcpx-rust fallback mention if stale; ensure README has no mcpx wording.
- Remove dead shims `~/.local/bin/{spec,map,ground,blast,impact}`.
- Final `sc_todo_complete` reply tells the agent to call `sc_plan` to archive.
- Release: bump version, publish to crates.io and GitHub, upgrade local install.
### Out-of-Scope
- Functional changes to the MCP tools; changes to `project-map` binary.

## Technical Constraints
- **Language/Runtime**: Rust (cargo)
- **Data Storage**: Local JSON/Markdown files
- **Key Constraints**: `grep -rn mcpx src README.md tests` returns nothing; `cargo test` passes.

## User Stories
- As an agent using the spec MCP tools, I want hints that say "call sc_approve", so that I don't have to ignore mcpx wording.
  - [ ] No mcpx in src/README/tests
  - [ ] cargo test passes
  - [ ] Shims removed

## Overview & Architecture
String-only edits in spec_manager.rs plus one added hint; no architectural change.

## Technical Stack
- **Backend**: Rust
- **Frontend**: n/a
- **Utilities**: cargo test

## Components and Interfaces
`SpecManager` status/approve/todo reply text.

## Data Models
None changed.
