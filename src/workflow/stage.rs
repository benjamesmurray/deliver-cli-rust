use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Requirements,
    Design,
    Tasks,
    Completed,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Stage::Requirements => "requirements",
            Stage::Design => "design",
            Stage::Tasks => "tasks",
            Stage::Completed => "completed",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusType {
    NotStarted,
    NotEdited,
    InProgress,
    ReadyToConfirm,
    Confirmed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub r#type: StatusType,
    pub reason: Option<String>,
    pub ready_to_confirm: bool,
}
