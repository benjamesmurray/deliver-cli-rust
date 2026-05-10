use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "deliver-cli")]
#[command(about = "MCP specification workflow server", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MCP server
    Mcp,
    /// Get a health check of the active project
    Status {
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Verify current state and check consistency
    Verify {
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Initialize a new feature specification
    Init {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        mode: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extraneous_args: Vec<String>,
    },
    /// Progress the workflow state
    Plan {
        #[arg(short, long)]
        feature: Option<String>,
        #[arg(short, long)]
        instruction: Option<String>,
    },
    /// Explicitly approve the current drafted phase
    Approve {
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Manage implementation tasks
    Todo {
        #[command(subcommand)]
        action: TodoAction,
    },
    /// Update context for short-term memory
    Epoch {
        #[arg(short, long)]
        feature: Option<String>,
        #[arg(long)]
        focus: Option<String>,
        #[arg(long)]
        intentions: Option<String>,
        #[arg(long)]
        hypotheses: Option<String>,
        #[arg(long)]
        open_questions: Option<String>,
    },
    /// Manually move the project to the completed directory
    Archive {
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Toggle project mode
    Mode {
        mode: String,
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Provide user feedback or answers to open questions
    Feedback {
        instruction: String,
        #[arg(short, long)]
        feature: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TodoAction {
    /// List all tasks
    List {
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Mark a task as being actively worked on
    Start {
        #[arg(long)]
        id: String,
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Mark a task as completed
    Complete {
        #[arg(long)]
        id: String,
        #[arg(short, long)]
        feature: Option<String>,
    },
}
