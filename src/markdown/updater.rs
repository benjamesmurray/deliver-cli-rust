use regex::Regex;
use crate::markdown::parser::TaskParser;

pub struct MarkdownTaskUpdater;

impl MarkdownTaskUpdater {
    pub fn update_task_status(content: &str, task_id: &str, completed: bool) -> String {
        let status = if completed { 'x' } else { ' ' };
        Self::update_task_status_char(content, task_id, status)
    }

    pub fn update_task_status_char(content: &str, task_id: &str, status: char) -> String {
        let mut tasks_to_update = vec![task_id.to_string()];

        if status == 'x' && task_id.contains('.') {
            let flat_tasks = TaskParser::parse_flat(content);
            let mut current_id = task_id.to_string();

            while current_id.contains('.') {
                let parts: Vec<&str> = current_id.split('.').collect();
                let parent_id = parts[..parts.len() - 1].join(".");
                
                // Find siblings: tasks with same parent and same level
                let parent_parts_len = parent_id.split('.').count();
                let siblings: Vec<_> = flat_tasks.iter().filter(|t| {
                    let t_parts: Vec<&str> = t.id.split('.').collect();
                    t.id.starts_with(&format!("{}.", parent_id)) && t_parts.len() == parent_parts_len + 1
                }).collect();

                let all_siblings_done = siblings.iter().all(|t| {
                    if t.id == current_id {
                        true
                    } else {
                        t.completed
                    }
                });

                if all_siblings_done {
                    tasks_to_update.push(parent_id.clone());
                    current_id = parent_id;
                } else {
                    break;
                }
            }
        }

        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        for id in tasks_to_update {
            let re = Regex::new(&format!(r"(?P<indent>\s*-\s*\[)[ xX/](?P<suffix>\]\s+){}(?P<dot>\.?)(?P<rest>.*)$", regex::escape(&id))).unwrap();
            for line in lines.iter_mut() {
                if let Some(caps) = re.captures(line) {
                    let indent = &caps["indent"];
                    let suffix = &caps["suffix"];
                    let dot = &caps["dot"];
                    let rest = &caps["rest"];
                    *line = format!("{}{}{}{}{}{}", indent, status, suffix, id, dot, rest);
                    break;
                }
            }
        }

        lines.join("\n")
    }

    pub fn update_batch_task_status(content: &str, task_ids: &[String], completed: bool) -> String {
        let mut current_content = content.to_string();
        for id in task_ids {
            current_content = Self::update_task_status(&current_content, id, completed);
        }
        current_content
    }
}
