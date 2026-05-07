# deliver-cli

A streamlined Model Context Protocol (MCP) server and CLI tool for managing a specification-driven development workflow (Specification -> Implementation Planning) directly from your command line.

## Workflow

The CLI enforces a clean, two-phase process designed to minimize context bloat and keep AI agents focused:

1.  **Specification Phase**: Initialize the feature and draft a `Specification.md`. This combines high-level requirements and technical design into a single source of truth.
2.  **Implementation Planning**: Once the specification is approved, the system scaffolds `Tasks.md` for granular tracking of the build.

## Features

- **2-Phase Orchestration**: Simplified "Specification -> Tasks" workflow to reduce agent overhead and prevent looping.
- **MCP Server**: Exposes tools (`sc_init`, `sc_plan`, `sc_approve`, etc.) for AI agents to autonomously manage the project lifecycle.
- **Task Management**: Automatically tracks and updates task statuses (Pending `[ ]`, In Progress `[/]`, Completed `[x]`) within nested checklists.
- **Epoch Context**: Built-in short-term memory management for agents to track intentions, hypotheses, and open questions.
- **Approval Gates**: Requires explicit human approval via `sc_approve` before moving from Specification to Implementation.

## Installation

```bash
cargo install deliver-cli
```

## Usage

```bash
# 1. Initialize a new feature (Creates Specification.md)
deliver-cli init --name feature-name

# 2. Check project status and next steps
deliver-cli status

# 3. Draft/Edit the Specification.md
# (Remove all <template-specification> tags to mark as ready)

# 4. Approve the specification
deliver-cli approve

# 5. Scaffold implementation tasks (Creates Tasks.md)
deliver-cli plan

# 6. Start work on a specific task
deliver-cli todo start --id 1.1

# 7. Complete a task
deliver-cli todo complete --id 1.1
```

## Configuration

To use the MCP server, add it to your client's configuration file (e.g., `claude_desktop_config.json`):

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

## License

MIT
