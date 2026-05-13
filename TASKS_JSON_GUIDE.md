# Tasks.json Guide

As of `v0.1.10`, `deliver-cli` uses `Tasks.json` instead of `Tasks.md` for implementation planning. This change ensures that AI agents can reliably parse and update task states without the formatting ambiguities of Markdown.

## The Schema

Every `Tasks.json` file follows this strict structure:

```json
{
  "template_tags_present": true,
  "tasks": [
    {
      "id": "1",
      "title": "Foundation & Setup",
      "description": "Establish the project structure and shared types.",
      "status": "pending",
      "dependencies": []
    },
    {
      "id": "1.1",
      "title": "Initialize Repository",
      "description": "Run cargo init and setup .gitignore.",
      "status": "pending",
      "dependencies": []
    }
  ]
}
```

## Field Definitions

### `template_tags_present` (boolean)
*   **Purpose**: Acts as a safety gate for the drafting phase.
*   **Behavior**: When a project is first scaffolded, this is set to `true`. The `sc_approve` command will fail as long as this is `true`.
*   **Requirement**: You must set this to `false` once you have finished defining your task list.

### `tasks` (array)
A list of task objects that define the implementation plan.

#### `id` (string)
*   **Format**: Dot-notated numbers (e.g., `"1"`, `"1.1"`, `"1.1.1"`).
*   **Logic**: The CLI uses these IDs to determine hierarchy. A parent task (e.g., `"1"`) cannot be marked as `completed` until all its children (e.g., `"1.1"`, `"1.2"`) are also `completed`.

#### `title` (string)
*   **Purpose**: A short, descriptive name for the task.

#### `description` (string)
*   **Purpose**: Concrete implementation details. Include file paths, specific functions to write, or technical constraints here.

#### `status` (string)
*   **Allowed Values**: `"pending"`, `"in_progress"`, `"completed"`.
*   **Note**: While you can edit these manually, they are primarily managed by the tools `sc_todo_start` and `sc_todo_complete`.

#### `dependencies` (array of strings)
*   **Purpose**: A list of other task IDs that must be completed before this task can start.
*   **Note**: Currently used for documentation/agent guidance; prefix-based subtask rules are enforced by the CLI logic.

## Workflow for AI Agents

1.  **Scaffold**: When the user approves the Specification, the system creates `Tasks.json` with a single example task and `template_tags_present: true`.
2.  **Populate**: The agent should rewrite the `tasks` array with the full implementation plan.
3.  **Finalize**: The agent MUST set `"template_tags_present": false`.
4.  **Approve**: The human runs `deliver-cli approve` to lock in the plan.
5.  **Execute**: The agent uses `sc_todo_start(id="1.1")` to begin work, which automatically updates the JSON status.
