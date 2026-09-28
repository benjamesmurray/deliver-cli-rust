# domain-taxonomy-enhancements-and-mcp-schema-fix - Specification

Fix sc_init MCP tool schema, add post-init domain setting, support auto-discovery of domains, and bump to v0.1.20.

## Core Features & Scope

### In-Scope
1. **Fix `ScInit` MCP Tool Schema & Parameter Generation**:
   - In `src/mcp/server.rs`, remove `#[serde(flatten)] pub extra: HashMap<String, Value>` from `ScInit`.
   - Ensure the JSON schema derived by `macros::JsonSchema` exposes clean types:
     - `name`: string (required)
     - `description`: string (optional, nullable)
     - `mode`: string (optional, nullable)
     - `domain`: string (optional, nullable)
   - Eliminate `"type": "unknown"` and phantom `"required": ["extra"]`, ensuring MCP clients generate full parameter schemas instead of `parameters: null`.

2. **Post-Init Domain Assignment & Mutation (`deliver-cli domain` & `sc_domain`)**:
   - **CLI**:
     - `deliver-cli domain set <domain> [--feature <name>]`: updates the assigned domain for an active feature.
     - `deliver-cli domain list`: outputs available subsystem domains from configuration or auto-discovery.
     - `deliver-cli domain get [--feature <name>]`: displays the currently assigned domain.
   - **MCP Tool**:
     - `sc_domain`: accepts `domain: String` and optional `feature: Option<String>`.
   - **Persistence**:
     - Updates `domain` field in `.spec-meta.json`.
     - Updates or inserts `**Domain:** <domain>` in `.epoch-context.md`.
     - Reflects updated domain in `sc_status` and `deliver-cli status`.

3. **Subsystem Domain Auto-Discovery**:
   - In `SpecConfig::load_config(root_dir)`: if `domains` in `.spec.json` is empty or if `.spec.json` is missing, scan `projects/completed/` for immediate child directories matching the domain pattern (`\d{2}_.*` or numeric prefixed domains).
   - Populate `domains` with discovered entries so that domain validation and partitioned archival (`{domain}/{date}_{feature}`) work even when `.spec.json` is omitted.

4. **Domain Validation & Warning**:
   - When specifying `--domain` during `init` or `domain set`, check against known domains (configured or auto-discovered).
   - If a domain is unknown, log an advisory warning with suggested valid domains, but allow creation (permissive validation).

5. **Release & Packaging**:
   - Bump crate version in `Cargo.toml` and `Cargo.lock` from `0.1.19` to `0.1.20`.
   - Update `README.md` and `SC_INIT_GUIDE.md` documenting `sc_domain`, `domain set`, and `domain list`.
   - Provide explicit publish instructions (`cargo publish`) and local installation instructions (`cargo install --path . --force`).

### Out-of-Scope
- Automatically reorganizing historical directories in `projects/completed` (handled via standalone migration scripts).
- Modifying other MCP tools (`sc_status`, `sc_plan`, etc.) beyond status output reflection.

## Technical Constraints
- **Language & Runtime**: Rust (2024 edition), standard library `std::fs`, `std::path::PathBuf`.
- **MCP Framework**: `rust-mcp-sdk` v0.9.0 with derive macros.
- **Safety**: Modifying active feature metadata must not clobber other metadata fields in `.spec-meta.json`.
- **Backward Compatibility**: Fully backward-compatible with existing un-partitioned and partitioned workflows.

## User Stories & Acceptance Criteria

### US1: Reliable MCP Tool Parameter Exposure
- *As an AI agent in Antigravity/Claude*, when I inspect or call `sc_init`, the tool parameters (`name`, `domain`, `description`, `mode`) are properly defined in JSON Schema rather than `parameters: null`.
- *AC 1.1*: `tools/list` returns `sc_init` with `properties` containing `name`, `description`, `mode`, `domain`.
- *AC 1.2*: Only `name` is marked as required.
- *AC 1.3*: Passing `domain` via MCP initializes `.spec-meta.json` with `"domain": "<domain>"`.

### US2: Post-Init Domain Assignment
- *As a developer or AI agent*, if a feature was initialized without a domain (or under the wrong domain), I can assign or update the domain without manual JSON editing.
- *AC 2.1*: `deliver-cli domain set <domain>` updates `.spec-meta.json` in the active feature directory.
- *AC 2.2*: `.epoch-context.md` is updated with `**Domain:** <domain>`.
- *AC 2.3*: `sc_domain(domain="...")` via MCP performs the same update.
- *AC 2.4*: Subsequent `deliver-cli status` and `sc_status` display the new domain.

### US3: Auto-Discovery of Existing Domain Folders
- *As a user working in a repo without a `.spec.json` (such as `/opt/wde`)*, existing domain folders in `projects/completed/` are automatically recognized as valid domains.
- *AC 3.1*: If `SpecConfig.domains` is empty, scan `projects/completed/` for directories matching `\d{2}_.*`.
- *AC 3.2*: `deliver-cli domain list` displays the auto-discovered domains.
- *AC 3.3*: Feature archival routes correctly to `projects/completed/<domain>/<date>_<feature>`.

## Components and Interfaces

### 1. `SpecManager` Methods (`src/manager/spec_manager.rs`)
```rust
impl SpecManager {
    pub fn set_domain(&self, base_dir: &Path, feature_name: Option<&str>, domain: &str) -> Result<String>;
    pub fn get_domain(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<Option<String>>;
    pub fn list_domains(&self, base_dir: &Path) -> Result<Vec<String>>;
}
```

### 2. CLI Commands (`src/cli/commands.rs`)
```rust
#[derive(Subcommand)]
pub enum Commands {
    // ...
    Domain {
        #[command(subcommand)]
        action: DomainAction,
    },
}

#[derive(Subcommand)]
pub enum DomainAction {
    Set {
        domain: String,
        #[arg(short, long)]
        feature: Option<String>,
    },
    Get {
        #[arg(short, long)]
        feature: Option<String>,
    },
    List,
}
```

### 3. MCP Tool (`src/mcp/server.rs`)
```rust
#[macros::mcp_tool(name = "sc_domain", description = "Set or update the subsystem domain for an active feature")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScDomain {
    pub domain: String,
    pub feature: Option<String>,
}
```

## Testing & Verification Plan
1. **Unit & Integration Tests**:
   - `test_mcp_init_schema_clean`: verify `tools/list` schema for `sc_init` has valid properties and only `name` required.
   - `test_domain_set_and_status`: verify `set_domain` updates `.spec-meta.json` and `.epoch-context.md`.
   - `test_domain_auto_discovery`: verify directories in `projects/completed/` are discovered when `.spec.json` is missing.
2. **End-to-End Archival Verification**:
   - Initialize feature -> `set_domain` -> approve spec -> complete tasks -> `plan` -> verify archive path contains `{domain}/{date}_{feature}`.
3. **Crate Release**:
   - Run `cargo test` and `cargo check`.
   - Publish to crates.io with `cargo publish`.
   - Upgrade local binary with `cargo install deliver-cli --force` and restart MCP server.