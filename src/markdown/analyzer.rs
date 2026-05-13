use std::fs;
use std::path::Path;

pub struct MarkdownAnalyzer;

impl MarkdownAnalyzer {
    pub fn is_edited(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if !path.exists() {
            return false;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        if path.extension().map_or(false, |ext| ext == "json") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                return v["template_tags_present"] != true;
            }
            return false;
        }

        !content.contains("<template-specification>") &&
        !content.contains("<template-tasks>")
    }
}
