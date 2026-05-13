# Feature Initialization Guide (`sc_init`)

The `sc_init` command is the starting point for any new feature in `deliver-cli`. It scaffolds the necessary directory structure and initial documentation to begin the Specification phase.

## Usage

You can call this command via the CLI or as an MCP tool.

### MCP Tool (for AI Agents)
```json
sc_init(
  name: "user-authentication",
  description: "Implement JWT-based login and registration",
  mode: "step-through"
)
```

### CLI (for Humans)
```bash
deliver-cli init --name "user-authentication" --description "Implement JWT-based login and registration"
```

## Parameters

### `name` (string, optional)
*   **Purpose**: Defines the folder name for the project.
*   **Behavior**:
    *   If provided, the project will be created at `projects/active/<name>/`.
    *   If omitted, a default name like `feature-1715587200` (timestamp) will be generated.
*   **Recommendation**: Use lowercase with hyphens for consistency (e.g., `api-migration`).

### `description` (string, optional)
*   **Purpose**: Provides context for the feature.
*   **Behavior**: This text is automatically interpolated into the `Project Background` section of the generated `Specification.md`.
*   **Importance**: A clear description helps the AI agent understand the high-level goal before it starts drafting the technical requirements.

### `mode` (string, optional)
*   **Allowed Values**: `"step-through"` (default), `"one-shot"`.
*   **Behavior**:
    *   `step-through`: Requires human approval (`sc_approve`) after each phase.
    *   `one-shot`: Designed for fully autonomous execution where the agent can move between phases without explicit blocking gates.

## What Happens During Initialization?

When `sc_init` is executed, the following actions occur:

1.  **Directory Creation**: Creates a new folder in `projects/active/` based on the `name`.
2.  **Specification Scaffolding**: 
    *   Generates `Specification.md` using the project's default template.
    *   Injects the `name` and `description` into the template variables.
3.  **Epoch Context**: Creates a `.epoch-context.md` file to track focus and open questions.
4.  **Mode Selection**: Persists the chosen `mode` into a hidden `.spec-mode` file.
5.  **Status Report**: Returns a `spec_status` summary indicating that the project is now in the `drafting` phase.

## Next Steps

After initialization, the project is in a `drafting` state. You (or your AI agent) must:
1.  Open `Specification.md`.
2.  Fill out the technical details and architecture.
3.  **Remove all `<template-specification>` tags.**
4.  Run `sc_status` to verify the project has moved to the `reviewing` state.
