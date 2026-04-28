# deliver-cli

A streamlined Model Context Protocol (MCP) server and CLI tool for managing a specification-driven development workflow (Requirements -> Design -> Implementation) directly from your command line.

## Features

- **Workflow Orchestration:** Define and enforce structured development phases using Markdown.
- **MCP Server:** Exposes tools for AI agents (like Claude or Cursor) to automatically scaffold, edit, and verify project specifications.
- **Task Management:** Automatically tracks and updates task completion statuses within nested checklists.
- **Approval Gates:** Ensures human review before advancing to the next implementation phase.

## Installation

```bash
cargo install deliver-cli
```

## Usage

```bash
# Initialize a new feature
deliver-cli init --name feature-name

# Check project status
deliver-cli status

# Start a task
deliver-cli todo start --id 1.1

# Complete a task
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
