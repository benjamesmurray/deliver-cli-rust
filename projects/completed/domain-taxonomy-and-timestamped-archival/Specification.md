# domain-taxonomy-and-timestamped-archival - Specification

Native Domain Taxonomy & Timestamped Archival in Spec Lifecycle (sc_init & sc_plan)

## Core Features & Scope

### In-Scope
- **Repo-Level Domain Configuration (`.spec.json`)**:
  - Support repository configuration file `.spec.json` (and `.antigravity.md` fallback or metadata if applicable) at the project root.
  - Configuration properties:
    - `completed_directory`: Directory path where completed features are archived (defaults to `projects/completed`).
    - `archive_format`: String template for archive destination paths (defaults to `{domain}/{date}_{feature}` when domains are configured, or `{date}_{feature}` when not configured).
    - `domains`: List of recognized domain/subsystem strings (e.g., `["01_level_detection_and_swings", ...]`).
- **Domain Assignment & Feature Metadata (`sc_init`)**:
  - Add optional `--domain` / `domain` parameter to CLI `init` and MCP `sc_init` tool schema.
  - Persist feature metadata upon initialization in `.spec-meta.json` inside the feature directory:
    - `feature`: string (feature name)
    - `domain`: optional string (assigned domain)
    - `created_at`: string (ISO 8601 UTC timestamp)
  - Validate or normalize domain against configured domains if repository configuration is present.
- **Automated Timestamped Archival (`sc_plan` / `archive`)**:
  - When archiving a completed feature (via `sc_plan` Phase 4 auto-advance or CLI `archive`):
    - Compute `completion_date` as `YYYY-MM-DD` (UTC).
    - Read `domain` from `.spec-meta.json` (if missing, fallback to `"00_general"` if domains configured, or omitted if domains not configured).
    - Resolve target path based on `completed_directory` and `archive_format`.
    - Recursively create destination parent directory.
    - Perform atomic move/rename of the feature directory from active to completed location.
    - Update `.spec_last_used` to point to the newly archived path.
- **Enhanced Feature Path Resolution**:
  - Update `resolve_feature_path` to scan nested domain folders within `projects/completed` (and match prefixed date directories such as `<YYYY-MM-DD>_<feature>`) so historical features remain discoverable by name.
- **Backwards Compatibility**:
  - If `.spec.json` does not exist or `domains` is empty, fallback gracefully to archiving to `projects/completed/<YYYY-MM-DD>_<feature>`.
  - Existing features without `.spec-meta.json` continue to function without error.

### Out-of-Scope
- Migration tool for pre-existing archived projects in older flat formats.
- Interactive CLI prompt wizard when `--domain` is omitted (interactive tty prompt is not suitable for headless MCP; fallback default or CLI flag is used).

## Technical Constraints
- **Language/Runtime**: Rust 2024 edition (`cargo` toolchain).
- **Serialization**: `serde` / `serde_json` for `.spec.json` and `.spec-meta.json`.
- **Date/Time**: `chrono` (already a project dependency).
- **Filesystem**: Atomic `fs::rename` with fallback to `fs_extra::dir::move_dir` for cross-device moves.
- **Backward Compatibility**: Fully compatible with existing MCP callers and CLI scripts where `domain` argument is not supplied.

## User Stories
- **US1: Domain Config**: As a repository maintainer, I want to define standard subsystem domains and archive path format in `.spec.json`, so that archived specs are neatly categorized by subsystem.
- **US2: Domain Assignment on Init**: As a developer running `sc_init`, I want to supply a `domain` (e.g. `domain="05_backtest_analyser_and_scenarios"`), so that my feature is tagged from inception in `.spec-meta.json`.
- **US3: Timestamped Archival**: As a developer completing a feature with `sc_plan()`, I want the completed feature automatically moved to `projects/completed/<domain>/<YYYY-MM-DD>_<feature>`, preserving chronological context and clean directory layout.
- **US4: Legacy Graceful Fallback**: As a developer in a repository without configured domains, I want `sc_plan()` to archive to `projects/completed/<YYYY-MM-DD>_<feature>` without requiring a `.spec.json`.
- **US5: Historical Discovery**: As a developer or agent checking `sc_status` on an archived feature by name, I want `deliver-cli` to locate it even inside a domain subfolder with a date prefix.

## Overview & Architecture
```
deliver-cli-rust
├── src/
│   ├── cli/commands.rs         --> Add --domain to Init command
│   ├── mcp/server.rs           --> Add domain field to ScInit MCP tool
│   ├── manager/
│   │   └── spec_manager.rs     --> SpecConfig loading, .spec-meta.json persistence,
│   │                               timestamped/domain archival logic, nested completed discovery
```

### Flow:
1. `init(name, description, mode, domain)`:
   - Scaffolds feature directory.
   - Generates `.spec-meta.json` with `{ feature, domain, created_at }`.
   - Generates `Specification.md` and `.epoch-context.md`.
2. `archive(feature_name)`:
   - Reads `.spec.json` from project root if present; otherwise uses default config.
   - Reads `.spec-meta.json` from feature directory if present.
   - Computes target path: `completed_directory / format(archive_format, domain, date, feature)`.
   - Moves directory and updates `.spec_last_used`.
3. `resolve_feature_path`:
   - Checks active directories.
   - Checks completed directories, searching top-level and domain subdirectories, matching exact name or `<date>_<name>`.

## Technical Stack
- **Rust Dependencies**:
  - `serde`, `serde_json` for config and metadata handling.
  - `chrono` for UTC date and RFC 3339 timestamps.
  - `clap` for CLI argument parsing.
  - `rust-mcp-sdk` for MCP tool declaration.
- **Testing**:
  - Integration tests in `tests/workflow_test.rs` and `tests/init_test.rs`.

## Components and Interfaces

### 1. `SpecConfig` (`src/manager/spec_manager.rs` or `src/manager/config.rs`)
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecConfig {
    #[serde(default = "default_completed_directory")]
    pub completed_directory: String,
    #[serde(default = "default_archive_format")]
    pub archive_format: String,
    #[serde(default)]
    pub domains: Vec<String>,
}
```

### 2. `SpecMeta` (`src/manager/spec_manager.rs`)
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecMeta {
    pub feature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub created_at: String,
}
```

### 3. CLI & MCP Tool Interface
- CLI: `deliver-cli init --name <name> [--domain <domain>] [--description <desc>] [--mode <mode>]`
- MCP: `sc_init(name: String, description?: String, mode?: String, domain?: String)`

## Data Models
- `.spec.json` schema:
```json
{
  "completed_directory": "projects/completed",
  "archive_format": "{domain}/{date}_{feature}",
  "domains": [
    "01_level_detection_and_swings",
    "02_regimes_and_phases"
  ]
}
```
- `.spec-meta.json` schema:
```json
{
  "feature": "backtest-fast-cache",
  "domain": "05_backtest_analyser_and_scenarios",
  "created_at": "2026-09-04T11:00:00Z"
}
```