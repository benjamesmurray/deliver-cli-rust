# auto-prune-empty-parents-and-clean-abort - Specification

Auto-Prune Empty Parent Directories on Archive & Add sc_clean / sc_abort MCP Tools

## Core Features & Scope

### In-Scope
- **Auto-Prune Empty Parent Folders on Archive**:
  - In `SpecManager::archive`, after moving `current_path` to `target_path`, check if `current_path` had parent directories inside `projects/active`.
  - Ascend up the directory hierarchy within `projects/active` (without removing `projects/active` itself) and delete any empty parent directories.
- **SpecManager::clean**:
  - Recursively traverse `projects/active` and delete any empty subdirectories (bottom-up traversal).
  - Return a summary of cleaned directory paths or indicate if none were found.
- **SpecManager::abort**:
  - Resolve the target active feature (by explicit name or via `.spec_last_used`).
  - Verify that the target feature directory is located inside `projects/active` (reject aborting features outside `projects/active` or already completed).
  - Recursively remove the feature directory (`fs::remove_dir_all`).
  - If `.spec_last_used` points to the aborted feature, remove or clear `.spec_last_used`.
  - Invoke clean logic to prune any empty parent folders left behind.
  - Return a success message.
- **CLI Commands**:
  - Add `Commands::Clean` and `Commands::Abort { feature: Option<String> }` in `src/cli/commands.rs`.
  - Dispatch both commands in `src/main.rs`.
- **MCP Server Tools**:
  - Define `ScClean` and `ScAbort` MCP tool structs in `src/mcp/server.rs`.
  - Expose `sc_clean` and `sc_abort` in `handle_list_tools_request`.
  - Handle execution of `sc_clean` and `sc_abort` in `handle_call_tool_request`.
- **Crate Version & Testing**:
  - Bump crate version in `Cargo.toml` (0.1.18 -> 0.1.19).
  - Add unit and integration tests covering empty parent pruning on archive, `clean`, and `abort`.

### Out-of-Scope
- Deleting or modifying directories outside `projects/active` (e.g. `projects/completed` or repository root).
- Pruning directories that still contain active feature files or subdirectories.

## Technical Constraints
- **Language & Runtime**: Rust (2024 edition), standard library `std::fs`, `std::path::PathBuf`.
- **Security & Safety**: Directory climbing must strictly bound at `projects/active` and never delete `projects/active` itself or any path outside it.
- **Protocol**: MCP protocol tool schema adherence via `rust-mcp-sdk`.
- **Backward Compatibility**: Existing CLI commands and MCP tools must remain intact.

## User Stories & Acceptance Criteria
- **US1: Clean Archive**:
  - *As a developer/agent*, when I archive a nested feature (e.g. `projects/active/06_domain/my-feature`), the empty domain directory `06_domain` is removed so that active features do not leave ghost directories.
  - *AC 1.1*: Archiving a nested feature removes all empty ancestors up to `projects/active`.
  - *AC 1.2*: If an ancestor directory still contains other active features, it is NOT removed.
- **US2: Active Feature Abort**:
  - *As a developer/agent*, when I need to abandon a feature or cancel a faulty initialization, I can call `sc_abort` or `deliver-cli abort`.
  - *AC 2.1*: The feature folder is completely removed.
  - *AC 2.2*: If `.spec_last_used` points to this feature, it is removed.
  - *AC 2.3*: Empty parent folders inside `projects/active` are pruned.
  - *AC 2.4*: Features outside `projects/active` cannot be aborted.
- **US3: Active Directory Clean**:
  - *As a developer/agent*, I can run `sc_clean` or `deliver-cli clean` to prune any orphaned empty directories in `projects/active`.
  - *AC 3.1*: All empty directories within `projects/active` are pruned bottom-up.
  - *AC 3.2*: Non-empty directories remain untouched.

## Overview & Architecture
```
User / MCP Client
    │
    ├─> CLI Commands: [Archive, Clean, Abort] (src/cli/commands.rs -> src/main.rs)
    │
    └─> MCP Tools:    [sc_clean, sc_abort]   (src/mcp/server.rs)
                            │
                            ▼
                     SpecManager (src/manager/spec_manager.rs)
                     ├─ archive() + prune empty ancestors
                     ├─ clean() (bottom-up empty dir pruning)
                     └─ abort() (remove dir, reset last_used, prune empty ancestors)
```

## Technical Stack
- **Language**: Rust
- **Dependencies**: `anyhow`, `clap`, `serde`, `serde_json`, `rust-mcp-sdk`, `chrono`, `fs_extra`
- **Testing**: `cargo test`, `tempfile`

## Components and Interfaces

### SpecManager
```rust
impl SpecManager {
    // Modified:
    pub fn archive(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<String>;

    // New:
    pub fn clean(&self, base_dir: &Path) -> Result<String>;
    pub fn abort(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<String>;
}
```

### CLI
```rust
pub enum Commands {
    // ...
    /// Remove empty directories from projects/active
    Clean,
    /// Abort and remove an active feature specification
    Abort {
        #[arg(short, long)]
        feature: Option<String>,
    },
}
```

### MCP Server
```rust
#[macros::mcp_tool(name = "sc_clean", description = "Prune empty directories in projects/active")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScClean {}

#[macros::mcp_tool(name = "sc_abort", description = "Abort and remove an active feature")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScAbort {
    pub feature: Option<String>,
}
```

## Verification & Testing Plan
1. **Unit & Integration Tests (`tests/workflow_test.rs` or new tests)**:
   - Test pruning on empty nested active directories.
   - Test `archive()` does not leave empty parent folders behind.
   - Test `abort()` deletes the directory, clears `.spec_last_used`, and cleans up parent directories.
   - Test `abort()` rejects non-active features.
   - Test `clean()` prunes nested empty directories without removing directories that contain files.
2. **Build Verification**:
   - `cargo test` passes all existing and new tests.
   - `cargo build --release` compiles without errors or warnings.