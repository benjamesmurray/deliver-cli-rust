use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OpenApiSpec {
    pub info: Info,
    #[serde(rename = "x-global-config")]
    pub global_config: GlobalConfig,
    #[serde(rename = "x-document-templates")]
    pub document_templates: HashMap<String, DocumentTemplate>,
    #[serde(rename = "x-shared-resources")]
    pub shared_resources: HashMap<String, SharedResource>,
    #[serde(rename = "x-task-guidance-template")]
    pub task_guidance_template: Option<TaskGuidanceTemplate>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Info {
    pub title: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GlobalConfig {
    pub stage_names: HashMap<String, String>,
    pub file_names: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DocumentTemplate {
    pub title: String,
    pub sections: Vec<TemplateSection>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TemplateSection {
    pub name: String,
    pub content: Option<String>,
    pub placeholder: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SharedResource {
    pub uri: String,
    pub title: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TaskGuidanceTemplate {
    pub separator: String,
    pub header: String,
    pub instructions: HashMap<String, String>,
    pub prompts: HashMap<String, String>,
    #[serde(rename = "completionMessages")]
    pub completion_messages: HashMap<String, String>,
}

pub struct OpenApiLoader {
    pub spec: OpenApiSpec,
}

impl OpenApiLoader {
    pub fn load(spec_path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(spec_path)?;
        let spec: OpenApiSpec = serde_yaml::from_str(&content)?;
        Ok(Self { spec })
    }

    pub fn load_default() -> Result<Self> {
        let content = include_str!("../assets/default-spec.yaml");
        let spec: OpenApiSpec = serde_yaml::from_str(content)?;
        Ok(Self { spec })
    }

    pub fn empty() -> Self {
        Self {
            spec: OpenApiSpec::default(),
        }
    }

    pub fn get_file_name(&self, stage: &str) -> Option<&String> {
        self.spec.global_config.file_names.get(stage)
    }

    pub fn get_template(&self, stage: &str) -> Option<&DocumentTemplate> {
        self.spec.document_templates.get(stage)
    }

    pub fn get_shared_resource(&self, uri: &str) -> Option<&String> {
        self.spec.shared_resources.get(uri).and_then(|r| r.text.as_ref())
    }
}
