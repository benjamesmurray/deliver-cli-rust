use std::fs;
use std::path::Path;

pub struct MarkdownAnalyzer;

impl MarkdownAnalyzer {
    pub fn is_edited(path: impl AsRef<Path>) -> bool {
        if !path.as_ref().exists() {
            return false;
        }

        match fs::read_to_string(path) {
            Ok(content) => {
                !content.contains("<template-specification>") &&
                !content.contains("<template-tasks>")
            }
            Err(_) => false,
        }
    }
}
