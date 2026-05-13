# deliver-cli

A streamlined Model Context Protocol (MCP) server and CLI tool for managing a specification-driven development workflow (Specification -> Implementation Planning) directly from your command line.

## Workflow Overview

The CLI enforces a clean, two-phase process designed to minimize context bloat and keep AI agents focused:

1.  **Specification Phase**: Initialize the feature and draft a `Specification.md`. This combines high-level requirements and technical design into a single source of truth.
2.  **Implementation Planning**: Once the specification is approved, the system scaffolds `Tasks.json` for granular tracking of the build.

## Tools: CLI vs MCP

There are two ways to interact with `deliver-cli`: via the **CLI** (for humans) or via **MCP Tools** (for AI agents).

| CLI Command | MCP Tool | Purpose |
| :--- | :--- | :--- |
| `deliver-cli init` | `sc_init` | Initialize a new feature and scaffold `Specification.md` |
| `deliver-cli status` | `sc_status` | Check the current phase, status, and next steps |
| `deliver-cli approve` | `sc_approve` | Approve current phase and **auto-scaffold next phase** |
| `deliver-cli plan` | `sc_plan` | Advance workflow or provide implementation instructions |
| `deliver-cli todo start --id <id>` | `sc_todo_start` | Mark a specific task as "In Progress" |
| `deliver-cli todo complete --id <id>` | `sc_todo_complete` | Mark a specific task as "Completed" |

## Example End-to-End Workflow

Here is the exact sequence an AI agent or human would follow to complete a project called `user-auth`:

### Phase 1: Specification
1.  **Initialize**: `sc_init(name="user-auth", description="Add JWT auth")`
    *   Creates `projects/active/user-auth/Specification.md`.
2.  **Draft**: The agent writes the technical requirements and architecture into `Specification.md`.
3.  **Check Status**: `sc_status()`
    *   Shows `phase: specification`, `status: drafting`.
    *   *Agent must remove all `<template-specification>` tags from the file to proceed.*
4.  **Review**: Once tags are removed, `sc_status()` shows `status: reviewing`.
5.  **Approve & Scaffold**: The human reviews the doc and runs `deliver-cli approve` (or the agent calls `sc_approve()`).
    *   **New**: This automatically scaffolds `projects/active/user-auth/Tasks.json`.

### Phase 2: Implementation Planning
6.  **Define Tasks**: The agent populates `Tasks.json` with a structured list of tasks.
7.  **Final Approval**: Once tasks are edited and `"template_tags_present": false` is set in the JSON, run `sc_approve()` again to transition to the implementation phase.

### Phase 3: Build
9.  **Start Task**: `sc_todo_start(id="1.1")`
    *   Updates task status in `Tasks.json` to `"in_progress"`.
10. **Build**: The agent performs the actual coding work.
11. **Complete Task**: `sc_todo_complete(id="1.1")`
    *   Updates task status in `Tasks.json` to `"completed"`.
12. **Archive**: Once all tasks are `"completed"`, `sc_plan()` will automatically move the project to `projects/completed/`.

## Features

- **2-Phase Orchestration**: Simplified "Specification -> Tasks" workflow to reduce agent overhead.
- **JSON Task Management**: Deterministic task tracking using `Tasks.json` to eliminate LLM parsing errors.
- **Epoch Context**: Short-term memory management via `deliver-cli epoch` to track focus, intentions, and open questions.
- **Approval Gates**: Requires explicit human approval before advancing from Specification to Implementation.

## Installation

```bash
cargo install deliver-cli
```

## Configuration

Add the server to your MCP client configuration (e.g., `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "deliver": {
      "command": "deliver-cli",
      "args": ["mcp"]
    }
  }
}
```

### Customizing Templates

By default, `deliver-cli` comes with built-in templates for `Specification.md` and `Tasks.md`. However, you can completely override these templates and the workflow text by providing your own OpenAPI YAML configuration file.

To use a custom configuration, set the `SPEC_PATH` environment variable before running the CLI or the MCP server:

```bash
export SPEC_PATH=/path/to/your/custom-workflow.yaml
deliver-cli status
```

In your MCP client configuration, you can pass this as an environment variable:

```json
{
  "mcpServers": {
    "deliver": {
      "command": "deliver-cli",
      "args": ["mcp"],
      "env": {
        "SPEC_PATH": "/absolute/path/to/your/custom-workflow.yaml"
      }
    }
  }
}
```

Your custom YAML file must define the `x-document-templates` block (containing `specification` and `tasks` templates) to dictate how files are scaffolded. The CLI detects whether a document is still in draft form by looking for `<template-specification>` and `<template-tasks>` tags within those files.

## License

MIT
