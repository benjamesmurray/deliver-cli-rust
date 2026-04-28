use std::collections::HashMap;
use crate::io::openapi_loader::DocumentTemplate;

pub struct TemplateEngine;

impl TemplateEngine {
    pub fn interpolate(template: &DocumentTemplate, variables: &HashMap<String, String>) -> String {
        let mut output = String::new();

        // Interpolate title
        let title = Self::replace_vars(&template.title, variables);
        output.push_str(&format!("# {}\n\n", title));

        for section in &template.sections {
            if let Some(content) = &section.content {
                output.push_str(&Self::replace_vars(content, variables));
            } else if let Some(placeholder) = &section.placeholder {
                output.push_str(&Self::replace_vars(placeholder, variables));
            }
            output.push_str("\n\n");
        }

        output.trim().to_string()
    }

    fn replace_vars(text: &str, variables: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (key, value) in variables {
            let pattern = format!("${{{}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }
}
