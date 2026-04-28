use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub completed: bool,
    pub line: usize,
    pub children: Vec<Task>,
}

pub struct TaskParser;

impl TaskParser {
    pub fn parse(content: &str) -> Vec<Task> {
        let flat_tasks = Self::parse_flat(content);
        Self::build_hierarchy(flat_tasks)
    }

    pub fn parse_flat(content: &str) -> Vec<Task> {
        let mut flat_tasks = Vec::new();
        let re = Regex::new(r"^(\s*)-\s*\[([ xX])\]\s+(\d+(?:\.\d+)*)\.?\s*(.*)$").unwrap();

        for (i, line) in content.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let completed = caps[2].to_lowercase() == "x";
                let id = caps[3].to_string();
                let text = caps[4].trim().to_string();

                flat_tasks.push(Task {
                    id,
                    text,
                    completed,
                    line: i + 1,
                    children: Vec::new(),
                });
            }
        }
        flat_tasks
    }

    fn build_hierarchy(flat_tasks: Vec<Task>) -> Vec<Task> {
        let mut task_map: std::collections::HashMap<String, Task> = flat_tasks
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect();

        let mut root_ids = Vec::new();
        let mut children_to_assign = Vec::new();

        for id in task_map.keys() {
            let parts: Vec<&str> = id.split('.').collect();
            if parts.len() == 1 {
                root_ids.push(id.clone());
            } else {
                let parent_id = parts[..parts.len() - 1].join(".");
                children_to_assign.push((parent_id, id.clone()));
            }
        }

        // Sort children_to_assign to process deeper levels first? 
        // No, we need to assign to parents.
        for (parent_id, child_id) in children_to_assign {
            if let Some(child) = task_map.remove(&child_id) {
                if let Some(parent) = task_map.get_mut(&parent_id) {
                    parent.children.push(child);
                } else {
                    // Parent not found, put it back as a root
                    task_map.insert(child_id.clone(), child);
                    root_ids.push(child_id);
                }
            }
        }

        let mut roots: Vec<Task> = task_map.into_values().collect();
        roots.sort_by_key(|t| t.line);
        
        // Recursively sort children
        for root in &mut roots {
            Self::sort_children(root);
        }

        roots
    }

    fn sort_children(task: &mut Task) {
        task.children.sort_by_key(|t| t.line);
        for child in &mut task.children {
            Self::sort_children(child);
        }
    }
}

pub struct TaskPresenter;

impl TaskPresenter {
    pub fn format_full_display(tasks: &[Task]) -> String {
        let mut output = String::new();
        for task in tasks {
            Self::format_task(task, 0, &mut output);
        }
        output.trim().to_string()
    }

    fn format_task(task: &Task, level: usize, output: &mut String) {
        let indent = "  ".repeat(level);
        let checkbox = if task.completed { "[x]" } else { "[ ]" };
        output.push_str(&format!("{}{}. {} {}\n", indent, task.id, checkbox, task.text));
        for child in &task.children {
            Self::format_task(child, level + 1, output);
        }
    }
}
