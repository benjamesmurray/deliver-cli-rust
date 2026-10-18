use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;
use crate::markdown::analyzer::MarkdownAnalyzer;
use crate::io::openapi_loader::OpenApiLoader;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TasksJson {
    #[serde(default)]
    pub template_tags_present: bool,
    pub tasks: Vec<TaskDef>,
}

fn default_completed_directory() -> String {
    "projects/completed".to_string()
}

fn default_archive_format() -> String {
    "{domain}/{date}_{feature}".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SpecConfig {
    #[serde(default = "default_completed_directory")]
    pub completed_directory: String,
    #[serde(default = "default_archive_format")]
    pub archive_format: String,
    #[serde(default)]
    pub domains: Vec<String>,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            completed_directory: default_completed_directory(),
            archive_format: default_archive_format(),
            domains: Vec::new(),
        }
    }
}

impl SpecConfig {
    pub fn discover_domains(completed_dir: &Path) -> Vec<String> {
        let mut domains = Vec::new();
        if let Ok(entries) = fs::read_dir(completed_dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_domain = name.split_once('_').map(|(prefix, _)| {
                            prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() && prefix.len() <= 3
                        }).unwrap_or(false);
                        if is_domain {
                            domains.push(name);
                        }
                    }
                }
            }
        }
        domains.sort();
        domains
    }

    pub fn compute_archive_rel_path(&self, feature: &str, domain: Option<&str>, date: &str) -> PathBuf {
        let has_domains_configured = !self.domains.is_empty();
        let resolved_domain = if has_domains_configured {
            domain.unwrap_or("00_general")
        } else {
            domain.unwrap_or("")
        };

        let mut format = self.archive_format.clone();
        if !has_domains_configured && domain.is_none() {
            format = format.replace("{domain}/", "").replace("{domain}", "");
            if format.starts_with('/') {
                format = format.trim_start_matches('/').to_string();
            }
            if format.is_empty() {
                format = "{date}_{feature}".to_string();
            }
        }

        let formatted = format
            .replace("{domain}", resolved_domain)
            .replace("{date}", date)
            .replace("{feature}", feature);

        PathBuf::from(&self.completed_directory).join(formatted.trim_start_matches('/'))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SpecMeta {
    pub feature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub created_at: String,
}

impl SpecMeta {
    pub fn load_from_dir(feature_path: &Path) -> Option<Self> {
        let meta_path = feature_path.join(SpecManager::META_FILE);
        if meta_path.exists() {
            if let Ok(content) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<SpecMeta>(&content) {
                    return Some(meta);
                }
            }
        }
        None
    }

    pub fn save_to_dir(&self, feature_path: &Path) -> Result<()> {
        let meta_path = feature_path.join(SpecManager::META_FILE);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(meta_path, content)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskDef {
    #[serde(deserialize_with = "deserialize_flexible_string")]
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String, // "pending", "in_progress", "completed"
    #[serde(deserialize_with = "deserialize_flexible_string_vec")]
    pub dependencies: Vec<String>,
}

pub fn deserialize_flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum FlexibleId {
        Int(i64),
        String(String),
    }

    Ok(match FlexibleId::deserialize(deserializer)? {
        FlexibleId::Int(i) => i.to_string(),
        FlexibleId::String(s) => s,
    })
}

pub fn deserialize_flexible_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum FlexibleId {
        Int(i64),
        String(String),
    }

    let vec: Vec<FlexibleId> = Vec::deserialize(deserializer)?;
    Ok(vec.into_iter().map(|id| match id {
        FlexibleId::Int(i) => i.to_string(),
        FlexibleId::String(s) => s,
    }).collect())
}

pub struct WorkflowState {
    pub specification: DocumentState,
    pub tasks: DocumentState,
    pub feature_path: PathBuf,
}

pub struct DocumentState {
    pub exists: bool,
    pub edited: bool,
    pub approved: bool,
}

pub struct SpecManager;

impl SpecManager {
    pub const LAST_USED_FILE: &'static str = ".spec_last_used";
    pub const CONFIG_FILE: &'static str = ".spec.json";
    pub const META_FILE: &'static str = ".spec-meta.json";

    pub fn load_config(root_dir: &Path) -> SpecConfig {
        let json_path = root_dir.join(Self::CONFIG_FILE);
        if json_path.exists() {
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(config) = serde_json::from_str::<SpecConfig>(&content) {
                    return Self::with_auto_discovery(root_dir, config);
                }
            }
        }
        let antigravity_path = root_dir.join(".antigravity.md");
        if antigravity_path.exists() {
            if let Ok(content) = fs::read_to_string(&antigravity_path) {
                if let Ok(config) = serde_json::from_str::<SpecConfig>(&content) {
                    return Self::with_auto_discovery(root_dir, config);
                }
            }
        }
        Self::with_auto_discovery(root_dir, SpecConfig::default())
    }

    fn with_auto_discovery(root_dir: &Path, mut config: SpecConfig) -> SpecConfig {
        if config.domains.is_empty() {
            let completed_path = root_dir.join(&config.completed_directory);
            config.domains = SpecConfig::discover_domains(&completed_path);
        }
        config
    }

    pub fn find_project_root(start_dir: &Path) -> PathBuf {
        let mut current = fs::canonicalize(start_dir).unwrap_or(start_dir.to_path_buf());
        loop {
            // Check in current, and its parent, just in case
            let candidates = [
                current.join(".spec_root"),
                current.join("Cargo.toml"),
                current.join(".git"),
                current.join("package.json"),
            ];
            
            if candidates.iter().any(|p| p.exists()) {
                return current;
            }
            if let Some(parent) = current.parent() {
                if current == parent { break; }
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        start_dir.to_path_buf()
    }

    pub fn resolve_feature_path(base_dir: &Path, feature_name: Option<&str>) -> Result<PathBuf> {
        let root_dir = Self::find_project_root(base_dir);

        if let Some(name) = feature_name {
            let mut resolved_path = if Path::new(name).is_absolute() {
                PathBuf::from(name)
            } else {
                root_dir.join(name)
            };

            if !resolved_path.exists() {
                let common_dirs = ["projects/active", "projects/completed", "active", "completed", "specs", "docs"];
                for dir in common_dirs {
                    let path = root_dir.join(dir).join(name);
                    if path.exists() {
                        resolved_path = path;
                        break;
                    }
                }
            }

            if !resolved_path.exists() {
                let config = Self::load_config(&root_dir);
                let completed_candidates = [
                    root_dir.join(&config.completed_directory),
                    root_dir.join("projects/completed"),
                    root_dir.join("completed"),
                ];
                for completed_dir in completed_candidates {
                    if let Some(archived_path) = Self::find_archived_feature(&completed_dir, name) {
                        resolved_path = archived_path;
                        break;
                    }
                }
            }

            if !resolved_path.exists() && !name.contains('/') {
                 resolved_path = root_dir.join("projects/active").join(name);
            }

            Self::set_last_used(&root_dir, &resolved_path)?;
            return Ok(resolved_path);
        }

        let last_used_path = root_dir.join(Self::LAST_USED_FILE);
        if last_used_path.exists() {
            let last_used = fs::read_to_string(last_used_path)?.trim().to_string();
            let full_path = root_dir.join(last_used);
            if full_path.exists() {
                return Ok(full_path);
            }
        }

        Err(anyhow!("Could not determine project context. Please provide a feature name."))
    }

    fn set_last_used(root_dir: &Path, feature_path: &Path) -> Result<()> {
        let relative_path = feature_path.strip_prefix(root_dir).unwrap_or(feature_path);
        fs::write(root_dir.join(Self::LAST_USED_FILE), relative_path.to_string_lossy().as_bytes())?;
        Ok(())
    }

    fn find_archived_feature(completed_dir: &Path, name: &str) -> Option<PathBuf> {
        if !completed_dir.exists() || !completed_dir.is_dir() {
            return None;
        }

        let date_prefix_suffix = format!("_{}", name);

        let is_match = |path: &Path| -> bool {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name == name || dir_name.ends_with(&date_prefix_suffix) {
                    return true;
                }
            }
            if let Some(meta) = SpecMeta::load_from_dir(path) {
                if meta.feature == name {
                    return true;
                }
            }
            false
        };

        if let Ok(entries) = fs::read_dir(completed_dir) {
            let mut subdirs = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if is_match(&path) {
                        return Some(path);
                    }
                    subdirs.push(path);
                }
            }

            for subdir in subdirs {
                if let Ok(nested_entries) = fs::read_dir(&subdir) {
                    for nested in nested_entries.flatten() {
                        let path = nested.path();
                        if path.is_dir() && is_match(&path) {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    pub fn get_workflow_state(feature_path: &Path, loader: &OpenApiLoader) -> WorkflowState {
        let get_doc_state = |stage: &str| {
            let file_name = loader.get_file_name(stage).cloned().unwrap_or(format!("{}.md", stage));
            let file_path = feature_path.join(file_name);
            let approved_path = feature_path.join(format!(".spec-{}-approved", stage));
            DocumentState {
                exists: file_path.exists(),
                edited: MarkdownAnalyzer::is_edited(&file_path),
                approved: approved_path.exists(),
            }
        };

        WorkflowState {
            specification: get_doc_state("specification"),
            tasks: get_doc_state("tasks"),
            feature_path: feature_path.to_path_buf(),
        }
    }

    pub fn get_mode(feature_path: &Path) -> String {
        let mode_file = feature_path.join(".spec-mode");
        if mode_file.exists() {
            fs::read_to_string(mode_file).unwrap_or_default().trim().to_string()
        } else {
            "step-through".to_string()
        }
    }

    pub fn set_mode(feature_path: &Path, mode: &str) -> Result<()> {
        fs::write(feature_path.join(".spec-mode"), mode)?;
        Ok(())
    }

    pub fn get_status_summary(&self, base_dir: &Path, feature_name: Option<&str>, loader: &OpenApiLoader) -> String {
        let feature_path = match Self::resolve_feature_path(base_dir, feature_name) {
            Ok(p) => p,
            Err(e) => return format!("spec_status:\n  phase: error\n  error: {}\n  next_step: call sc_init with name=\"your-feature\"", e),
        };

        let state = Self::get_workflow_state(&feature_path, loader);
        let mode = Self::get_mode(&feature_path);
        let root_dir = Self::find_project_root(base_dir);
        let config = Self::load_config(&root_dir);
        let tasks_file = loader.get_file_name("tasks").cloned().unwrap_or("Tasks.json".to_string());

        let is_archived = feature_path.starts_with(root_dir.join(&config.completed_directory)) ||
                         feature_path.starts_with(root_dir.join("projects/completed")) || 
                         feature_path.starts_with(root_dir.join("completed"));

        let mut all_tasks_complete = false;
        if state.tasks.exists && state.tasks.edited {
            let tasks_path = feature_path.join(&tasks_file);
            if let Ok(content) = fs::read_to_string(&tasks_path) {
                if tasks_file.ends_with(".json") {
                    if let Ok(tasks_json) = serde_json::from_str::<TasksJson>(&content) {
                        all_tasks_complete = !tasks_json.tasks.is_empty() && tasks_json.tasks.iter().all(|t| t.status == "completed");
                    }
                } else {
                    let tasks = crate::markdown::parser::TaskParser::parse(&content);
                    if !tasks.is_empty() {
                        all_tasks_complete = Self::are_all_tasks_done(&tasks);
                    }
                }
            }
        }

        let next_steps: String;
        let phase: &str;
        let mut status = "drafting";
        let mut blockers = Vec::new();

        if is_archived {
            phase = "completed";
            status = "archived";
            next_steps = "Feature workflow complete.".to_string();
        } else if !state.specification.exists {
            phase = "specification";
            next_steps = "call sc_init to initialize specification.".to_string();
        } else if !state.specification.edited {
            phase = "specification";
            status = "drafting";
            blockers.push("template_tags_present");
            let spec_file = loader.get_file_name("specification").map(|s| s.as_str()).unwrap_or("specification.md");
            next_steps = format!("Complete '{}' and run `sc_approve` to finalize.", spec_file);
        } else if !state.specification.approved {
            phase = "specification";
            status = "reviewing";
            if mode == "one-shot" {
                next_steps = "Resolve ambiguities then call sc_plan.".to_string();
            } else {
                next_steps = "Review, then call sc_approve.".to_string();
            }
        } else if !state.tasks.exists {
            phase = "specification";
            status = "approved";
            next_steps = "Specification Document approved. Proceeding to implementation planning (TASKS phase).".to_string();
        } else if !state.tasks.edited {
            phase = "tasks";
            status = "drafting";
            blockers.push("template_tags_present");
            next_steps = format!("Define tasks in '{}' and run `sc_approve` to finalize the plan.", tasks_file);
        } else if !state.tasks.approved {
            phase = "tasks";
            status = "reviewing";
            if mode == "one-shot" {
                next_steps = "Resolve ambiguities then call sc_todo_start.".to_string();
            } else {
                next_steps = "Review, then call sc_approve.".to_string();
            }
        } else if !all_tasks_complete {
            phase = "implementation";
            status = "active";
            next_steps = "Proceed with tasks by calling sc_todo_start.".to_string();
        } else {
            phase = "completed";
            status = "finished";
            next_steps = "Workflow complete.".to_string();
        }

        let mut epoch_info = String::new();
        let epoch_path = feature_path.join(".epoch-context.md");
        if epoch_path.exists() {
            if let Ok(content) = fs::read_to_string(epoch_path) {
                epoch_info = format!("\n  epoch_context: |\n    {}", content.trim().replace("\n", "\n    "));
            }
        }

        let rel_feature = feature_path.strip_prefix(&root_dir).unwrap_or(&feature_path).to_string_lossy();
        let domain_info = if let Some(meta) = SpecMeta::load_from_dir(&feature_path) {
            if let Some(d) = meta.domain {
                format!("\n  domain: {}", d)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!(
            "spec_status:\n  feature: {}{}\n  phase: {}\n  status: {}\n  next_step: {}\n  blockers: [{}]\n  mode: {}{}",
            rel_feature, domain_info, phase, status, next_steps, blockers.join(", "), mode, epoch_info
        )
    }

    pub fn validate_transition(feature_path: &Path) -> Result<()> {
        let epoch_path = feature_path.join(".epoch-context.md");
        if epoch_path.exists() {
            let content = fs::read_to_string(epoch_path)?;
            let re = Regex::new(r"(?s)## Open Questions / Uncertainties\n(.*?)(?:##|$)").unwrap();
            if let Some(caps) = re.captures(&content) {
                let questions_text = caps[1].trim();
                if !questions_text.is_empty() && questions_text != "*" && questions_text.to_lowercase() != "none" {
                    let lines: Vec<&str> = questions_text.split('\n').filter(|l| !l.trim().is_empty()).collect();
                    let has_real_questions = lines.iter().any(|l| {
                        let t = l.trim_start_matches(|c| c == '*' || c == ' ' || c == '-').trim();
                        !t.is_empty() && t.to_lowercase() != "none" && t != "*"
                    });
                    if has_real_questions {
                        return Err(anyhow!("Cannot advance while there are active open questions in the epoch context. Please resolve them by calling `sc_epoch` with openQuestions=\"None\"."));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn approve(&self, base_dir: &Path, feature_name: Option<&str>, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let state = Self::get_workflow_state(&feature_path, loader);
        
        let mut phase = String::new();
        if state.specification.exists && state.specification.edited && !state.specification.approved {
            phase = "specification".to_string();
        } else if state.tasks.exists && state.tasks.edited && !state.tasks.approved {
            phase = "tasks".to_string();
        }

        if phase.is_empty() {
            return Err(anyhow!("No phase is currently in a \"Reviewing\" state to be approved."));
        }

        Self::validate_transition(&feature_path)?;

        let feedback_marker = feature_path.join(".spec-last-feedback");
        if feedback_marker.exists() {
            let marker_time_str = fs::read_to_string(&feedback_marker)?;
            if let Ok(marker_time) = chrono::DateTime::parse_from_rfc3339(&marker_time_str) {
                let now = chrono::Utc::now();
                if now.signed_duration_since(marker_time).num_milliseconds() < 2000 {
                    return Err(anyhow!("Approval blocked: Recent feedback was recorded. To prevent misinterpretation of information as approval, you must wait for a separate turn and explicit user approval before calling sc_approve."));
                }
            }
            fs::remove_file(feedback_marker)?;
        }

        let approved_path = feature_path.join(format!(".spec-{}-approved", phase));
        fs::write(approved_path, chrono::Utc::now().to_rfc3339())?;
        
        let display_name = loader.spec.global_config.stage_names.get(&phase).cloned().unwrap_or(phase.clone());
        let mut approve_msg = format!("✅ Phase \"{}\" approved.", display_name);
        if phase == "specification" {
            approve_msg.push_str(" You are now in the TASKS phase.");
        }

        // Auto-advance to next phase
        match self.plan(base_dir, feature_name, None, loader) {
            Ok(plan_msg) => Ok(format!("{}\n\n{}", approve_msg, plan_msg)),
            Err(e) => Ok(format!("{}\n\n⚠️ Auto-advance failed: {}", approve_msg, e)),
        }
    }

    pub fn archive(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<String> {
        let root_dir = Self::find_project_root(base_dir);
        let current_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let config = Self::load_config(&root_dir);
        let completed_dir = root_dir.join(&config.completed_directory);
        
        if current_path.starts_with(&completed_dir) ||
           current_path.starts_with(root_dir.join("projects/completed")) ||
           current_path.starts_with(root_dir.join("completed")) {
            return Ok("Project is already in the completed directory.".to_string());
        }

        let feature_raw_name = current_path.file_name().ok_or_else(|| anyhow!("Invalid feature path"))?.to_string_lossy().to_string();
        let meta = SpecMeta::load_from_dir(&current_path);
        let feature = meta.as_ref().map(|m| m.feature.as_str()).unwrap_or(&feature_raw_name);
        let domain = meta.as_ref().and_then(|m| m.domain.as_deref());
        let completion_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let rel_target = config.compute_archive_rel_path(feature, domain, &completion_date);
        let target_path = root_dir.join(&rel_target);

        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        match fs::rename(&current_path, &target_path) {
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::Other || e.raw_os_error() == Some(18) => {
                fs_extra::dir::move_dir(&current_path, &target_path, &fs_extra::dir::CopyOptions::new())?;
            }
            Err(e) => return Err(e.into()),
        }

        // After moving current_path to target_path:
        // If current_path had a parent inside projects/active (and was not projects/active itself), prune if empty.
        let active_root = root_dir.join("projects/active");
        if let Some(parent) = current_path.parent() {
            let mut curr_parent = parent.to_path_buf();
            while curr_parent.starts_with(&active_root) && curr_parent != active_root {
                if let Ok(mut entries) = fs::read_dir(&curr_parent) {
                    if entries.next().is_none() {
                        let _ = fs::remove_dir(&curr_parent);
                    } else {
                        break; // Not empty, stop ascending
                    }
                } else {
                    break;
                }
                if let Some(up) = curr_parent.parent() {
                    curr_parent = up.to_path_buf();
                } else {
                    break;
                }
            }
        }

        let rel_target_str = target_path.strip_prefix(&root_dir).unwrap_or(&target_path);
        fs::write(root_dir.join(Self::LAST_USED_FILE), rel_target_str.to_string_lossy().as_bytes())?;
        Ok(format!("Successfully archived project to {}.", rel_target_str.to_string_lossy()))
    }

    pub fn clean(&self, base_dir: &Path) -> Result<String> {
        let root_dir = Self::find_project_root(base_dir);
        let active_dir = root_dir.join("projects/active");
        if !active_dir.exists() {
            return Ok("No projects/active directory found.".to_string());
        }

        let mut removed = Vec::new();
        fn prune_empty_dirs(dir: &Path, removed: &mut Vec<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir)?.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    prune_empty_dirs(&path, removed)?;
                    if fs::read_dir(&path)?.next().is_none() {
                        fs::remove_dir(&path)?;
                        removed.push(path);
                    }
                }
            }
            Ok(())
        }

        prune_empty_dirs(&active_dir, &mut removed)?;
        if removed.is_empty() {
            Ok("No empty active directories to clean.".to_string())
        } else {
            Ok(format!("Cleaned {} empty director(ies): {:?}", removed.len(), removed))
        }
    }

    pub fn abort(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<String> {
        let root_dir = Self::find_project_root(base_dir);
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let active_dir = root_dir.join("projects/active");

        if feature_path == active_dir || !feature_path.starts_with(&active_dir) {
            return Err(anyhow!("Cannot abort a feature that is not in projects/active."));
        }

        // Remove the feature folder
        fs::remove_dir_all(&feature_path)?;

        // Reset last_used if pointing to this feature
        let last_used_file = root_dir.join(Self::LAST_USED_FILE);
        if last_used_file.exists() {
            if let Ok(content) = fs::read_to_string(&last_used_file) {
                let last_used_str = content.trim();
                let last_used_path = root_dir.join(last_used_str);
                if last_used_path == feature_path || !last_used_path.exists() {
                    let _ = fs::remove_file(&last_used_file);
                }
            } else {
                let _ = fs::remove_file(&last_used_file);
            }
        }

        // Clean up empty parent directories if any
        self.clean(base_dir)?;

        Ok(format!("Successfully aborted and removed active feature: {}", feature_path.display()))
    }

    pub fn start_task(&self, base_dir: &Path, feature_name: Option<&str>, task_id: &str, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let tasks_file = loader.get_file_name("tasks").cloned().unwrap_or("Tasks.json".to_string());
        let tasks_path = feature_path.join(&tasks_file);

        if !tasks_path.exists() {
            return Err(anyhow!("{} file does not exist. Please complete writing the tasks document first.", tasks_file));
        }

        let content = fs::read_to_string(&tasks_path)?;

        if tasks_file.ends_with(".json") {
            let mut tasks_json: TasksJson = serde_json::from_str(&content)?;
            let task = tasks_json.tasks.iter_mut().find(|t| t.id == task_id)
                .ok_or_else(|| anyhow!("Task {} not found", task_id))?;
            task.status = "in_progress".to_string();
            let updated_content = serde_json::to_string_pretty(&tasks_json)?;
            fs::write(&tasks_path, updated_content)?;
        } else {
            let flat_tasks = crate::markdown::parser::TaskParser::parse_flat(&content);
            let _task = flat_tasks.iter().find(|t| t.id == task_id)
                .ok_or_else(|| anyhow!("Task {} not found", task_id))?;

            let updated_content = crate::markdown::updater::MarkdownTaskUpdater::update_task_status_char(&content, task_id, '/');
            fs::write(&tasks_path, updated_content)?;
        }

        Ok(format!("🚀 Task {} marked as IN PROGRESS.", task_id))
    }

    pub fn complete_task(&self, base_dir: &Path, feature_name: Option<&str>, task_id: &str, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let tasks_file = loader.get_file_name("tasks").cloned().unwrap_or("Tasks.json".to_string());
        let tasks_path = feature_path.join(&tasks_file);

        if !tasks_path.exists() {
            return Err(anyhow!("{} file does not exist. Please complete writing the tasks document first.", tasks_file));
        }

        let content = fs::read_to_string(&tasks_path)?;

        if tasks_file.ends_with(".json") {
            let mut tasks_json: TasksJson = serde_json::from_str(&content)?;
            
            // Subtask check
            let prefix = format!("{}.", task_id);
            if tasks_json.tasks.iter().any(|t| t.id.starts_with(&prefix) && t.status != "completed") {
                return Err(anyhow!("Task {} has uncompleted subtasks", task_id));
            }

            let task = tasks_json.tasks.iter_mut().find(|t| t.id == task_id)
                .ok_or_else(|| anyhow!("Task {} not found", task_id))?;
            
            if task.status == "completed" {
                return Ok(format!("ℹ️ Task {} is already completed.", task_id));
            }

            task.status = "completed".to_string();
            let updated_content = serde_json::to_string_pretty(&tasks_json)?;
            fs::write(&tasks_path, updated_content)?;
        } else {
            let flat_tasks = crate::markdown::parser::TaskParser::parse_flat(&content);

            let task = flat_tasks.iter().find(|t| t.id == task_id)
                .ok_or_else(|| anyhow!("Task {} not found", task_id))?;

            if task.completed {
                return Ok(format!("ℹ️ Task {} is already completed.", task_id));
            }

            let subtasks: Vec<_> = flat_tasks.iter().filter(|t| t.id.starts_with(&format!("{}.", task_id))).collect();
            if subtasks.iter().any(|t| !t.completed) {
                return Err(anyhow!("Task {} has uncompleted subtasks", task_id));
            }

            let updated_content = crate::markdown::updater::MarkdownTaskUpdater::update_task_status(&content, task_id, true);
            fs::write(&tasks_path, updated_content)?;
        }

        // Clear feedback marker if it exists
        let feedback_marker = feature_path.join(".spec-last-feedback");
        if feedback_marker.exists() {
            fs::remove_file(feedback_marker)?;
        }

        let mut msg = format!("✅ Task {} marked as completed!", task_id);
        if let Ok(updated) = fs::read_to_string(&tasks_path) {
            let all_done = match serde_json::from_str::<serde_json::Value>(&updated) {
                Ok(v) => v["tasks"].as_array().map_or(false, |ts| !ts.is_empty() && ts.iter().all(|t| t["status"] == "completed")),
                Err(_) => {
                    let flat = crate::markdown::parser::TaskParser::parse_flat(&updated);
                    !flat.is_empty() && flat.iter().all(|t| t.completed)
                }
            };
            if all_done {
                msg.push_str("\nAll tasks complete. Call sc_plan to archive.");
            }
        }
        Ok(msg)
    }

    pub fn set_domain(&self, base_dir: &Path, feature_name: Option<&str>, domain: &str, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        if !feature_path.exists() {
            return Err(anyhow!("Active feature directory does not exist: {:?}", feature_path));
        }

        let root = Self::find_project_root(base_dir);
        let config = Self::load_config(&root);
        if !config.domains.is_empty() && !config.domains.contains(&domain.to_string()) {
            eprintln!("⚠️ Warning: Domain '{}' is not recognized in taxonomy. Known domains: {:?}", domain, config.domains);
        }

        let mut meta = SpecMeta::load_from_dir(&feature_path).unwrap_or_else(|| SpecMeta {
            feature: feature_path.file_name().unwrap().to_string_lossy().to_string(),
            domain: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        meta.domain = Some(domain.to_string());
        meta.save_to_dir(&feature_path)?;

        let epoch_path = feature_path.join(".epoch-context.md");
        if epoch_path.exists() {
            let content = fs::read_to_string(&epoch_path)?;
            let updated = if content.contains("**Domain:**") {
                let re = regex::Regex::new(r"\*\*Domain:\*\*[^\n]*").unwrap();
                re.replace(&content, format!("**Domain:** {}", domain)).to_string()
            } else if content.contains("**Current Phase:**") {
                let re = regex::Regex::new(r"(\*\*Current Phase:\*\*[^\n]*)").unwrap();
                re.replace(&content, format!("$1\n**Domain:** {}", domain)).to_string()
            } else {
                format!("{}\n\n**Domain:** {}\n", content.trim_end(), domain)
            };
            fs::write(epoch_path, updated)?;
        }

        Ok(format!("✅ Domain set to '{}' for feature '{}'.\n\n{}", domain, meta.feature, self.get_status_summary(base_dir, feature_name, loader)))
    }

    pub fn get_domain(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<Option<String>> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let meta = SpecMeta::load_from_dir(&feature_path);
        Ok(meta.and_then(|m| m.domain))
    }

    pub fn list_domains(&self, base_dir: &Path) -> Result<Vec<String>> {
        let root = Self::find_project_root(base_dir);
        let config = Self::load_config(&root);
        Ok(config.domains)
    }

    pub fn init(&self, base_dir: &Path, name: String, description: Option<String>, mode: Option<String>, domain: Option<String>, loader: &OpenApiLoader) -> Result<String> {
        let feature_name = name;

        let feature_path = Self::resolve_feature_path(base_dir, Some(&feature_name))?;
        if !feature_path.exists() {
            fs::create_dir_all(&feature_path)?;
        }
        
        if let Some(m) = mode {
            Self::set_mode(&feature_path, &m)?;
        }

        // Persist domain and feature metadata in .spec-meta.json
        let meta = SpecMeta {
            feature: feature_name.clone(),
            domain: domain.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        meta.save_to_dir(&feature_path)?;

        let spec_file = loader.get_file_name("specification").cloned().unwrap_or("specification.md".to_string());
        let spec_path = feature_path.join(&spec_file);
        
        let msg: String;
        if !spec_path.exists() {
            let template = loader.get_template("specification").ok_or_else(|| anyhow!("Specification template not found"))?;
            let mut vars = HashMap::new();
            vars.insert("featureName".to_string(), feature_name.clone());
            vars.insert("introduction".to_string(), description.unwrap_or_else(|| "Initial specification".to_string()));
            
            let is_json = spec_file.ends_with(".json");
            let content = crate::io::template_engine::TemplateEngine::interpolate(template, &vars, is_json);
            fs::write(&spec_path, content)?;

            let domain_line = match &domain {
                Some(d) => format!("\n**Domain:** {}\n", d),
                None => String::new(),
            };
            fs::write(feature_path.join(".epoch-context.md"), format!("# Epoch Context\n\n**Current Phase:** Specification\n{}\n", domain_line))?;
            msg = format!("✅ Feature initialized. You are in PLAN mode. NEXT STEPS:\n1. Use 'read' to examine the generated '{}' template.\n2. Complete '{}' by removing all <template> tags.\n3. Run 'sc_approve' to finalize the specification.\nCRITICAL: DO NOT draft '{}' yet. You must approve the specification first.", 
                spec_file, spec_file, loader.get_file_name("tasks").map(|s| s.as_str()).unwrap_or("Tasks.json"));
        } else {
            msg = format!("ℹ️ Specification already exists at: {:?}", spec_path);
        }

        Ok(format!("{}\n\n{}", msg, self.get_status_summary(base_dir, Some(&feature_name), loader)))
    }

    pub fn plan(&self, base_dir: &Path, feature: Option<&str>, instruction: Option<&str>, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature)?;
        let state = Self::get_workflow_state(&feature_path, loader);
        let mode = Self::get_mode(&feature_path);

        let message: String;

        if !state.specification.exists {
             let name = feature.map(|s| s.to_string())
                .unwrap_or_else(|| feature_path.file_name().unwrap().to_string_lossy().to_string());
             return self.init(base_dir, name, None, None, None, loader);
        } else if !state.specification.edited {
            message = format!("Please finish editing {} (remove all <template> tags) before advancing.", loader.get_file_name("specification").unwrap());
        } else if !state.specification.approved && mode != "one-shot" {
            message = "Specification drafted but not yet approved. Please review and call `sc_approve` before advancing.".to_string();
        } else if !state.tasks.exists {
            if mode == "one-shot" {
                Self::validate_transition(&feature_path)?;
            }
            let template = loader.get_template("tasks").ok_or_else(|| anyhow!("Tasks template not found"))?;
            let mut vars = HashMap::new();
            vars.insert("featureName".to_string(), feature_path.file_name().unwrap().to_string_lossy().to_string());
            let tasks_file = loader.get_file_name("tasks").unwrap();
            let is_json = tasks_file.ends_with(".json");
            let mut content = crate::io::template_engine::TemplateEngine::interpolate(template, &vars, is_json);
            if !is_json {
                if let Some(ins) = instruction {
                    content.push_str(&format!("\n\n> **Guidance:** {}", ins));
                }
            }
            fs::write(feature_path.join(tasks_file), content)?;
            fs::write(feature_path.join(".epoch-context.md"), "# Epoch Context\n\n**Current Phase:** Implementation Planning\n\n")?;
            message = format!("You are now in the TASKS phase.\n\nNEXT STEPS:\n1. Use 'read' to examine the generated '{}' template.\n2. Define your atomic implementation tasks in '{}'.\n   Requirement: EACH task MUST have 'id' (numeric), 'title', 'description', and 'status'.\n3. Run 'sc_approve' to finalize the plan and unlock Build Mode.", 
                tasks_file, tasks_file);
        } else if !state.tasks.edited {
            message = format!("Please finish editing {} (remove all <template> tags) before advancing.", loader.get_file_name("tasks").unwrap());
        } else if !state.tasks.approved && mode != "one-shot" {
            message = "Tasks drafted but not yet approved. Please review and call `sc_approve` before advancing.".to_string();
        } else {
            let mut all_tasks_complete = false;
            let tasks_file = loader.get_file_name("tasks").unwrap();
            let tasks_path = feature_path.join(tasks_file);
            if tasks_path.exists() {
                let content = fs::read_to_string(&tasks_path)?;
                if tasks_file.ends_with(".json") {
                    if let Ok(tasks_json) = serde_json::from_str::<TasksJson>(&content) {
                        all_tasks_complete = !tasks_json.tasks.is_empty() && tasks_json.tasks.iter().all(|t| t.status == "completed");
                    }
                } else {
                    let tasks = crate::markdown::parser::TaskParser::parse(&content);
                    if !tasks.is_empty() {
                        all_tasks_complete = Self::are_all_tasks_done(&tasks);
                    }
                }
            }

            if !all_tasks_complete {
                let mut msg = "Not all implementation tasks are complete. Proceed by calling `sc_todo_start` or finish tasks manually.".to_string();
                if let Some(ins) = instruction {
                    msg.push_str(&format!("\n> Received instruction: {}", ins));
                }
                message = msg;
            } else {
                let msg = "Workflow is completely finished.".to_string();
                let archive_res = self.archive(base_dir, feature)?;
                return Ok(format!("{}\n\n{}\n\n{}", msg, archive_res, self.get_status_summary(base_dir, None, loader)));
            }
        }

        Ok(format!("{}\n\n{}", message, self.get_status_summary(base_dir, feature, loader)))
    }

    pub fn are_all_tasks_done(tasks: &[crate::markdown::parser::Task]) -> bool {
        tasks.iter().all(|t| t.completed && (t.children.is_empty() || Self::are_all_tasks_done(&t.children)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flexible_id_deserialization() {
        let json = r#"{
            "tasks": [
                {
                    "id": 1,
                    "title": "Task 1",
                    "description": "Integer ID",
                    "status": "pending",
                    "dependencies": [2]
                },
                {
                    "id": "2",
                    "title": "Task 2",
                    "description": "String ID",
                    "status": "pending",
                    "dependencies": ["1"]
                }
            ]
        }"#;
        let tasks: TasksJson = serde_json::from_str(json).unwrap();
        assert_eq!(tasks.tasks[0].id, "1");
        assert_eq!(tasks.tasks[0].dependencies[0], "2");
        assert_eq!(tasks.tasks[1].id, "2");
        assert_eq!(tasks.tasks[1].dependencies[0], "1");
    }

    #[test]
    fn test_spec_config_defaults_and_path_computation() {
        let default_config = SpecConfig::default();
        assert_eq!(default_config.completed_directory, "projects/completed");
        assert_eq!(default_config.archive_format, "{domain}/{date}_{feature}");
        assert!(default_config.domains.is_empty());

        // Without domains configured, fallback to projects/completed/<date>_<feature>
        let path = default_config.compute_archive_rel_path("fast-cache", None, "2026-09-04");
        assert_eq!(path, PathBuf::from("projects/completed/2026-09-04_fast-cache"));

        // With domains configured
        let domain_config = SpecConfig {
            completed_directory: "projects/completed".to_string(),
            archive_format: "{domain}/{date}_{feature}".to_string(),
            domains: vec![
                "01_level_detection".to_string(),
                "05_backtest_analyser".to_string(),
            ],
        };

        // Explicit domain
        let path_with_domain = domain_config.compute_archive_rel_path(
            "fast-cache",
            Some("05_backtest_analyser"),
            "2026-09-04",
        );
        assert_eq!(
            path_with_domain,
            PathBuf::from("projects/completed/05_backtest_analyser/2026-09-04_fast-cache")
        );

        // Omitted domain with domains configured -> fallback to 00_general
        let path_fallback = domain_config.compute_archive_rel_path("fast-cache", None, "2026-09-04");
        assert_eq!(
            path_fallback,
            PathBuf::from("projects/completed/00_general/2026-09-04_fast-cache")
        );
    }

    #[test]
    fn test_spec_meta_serialization() {
        let meta = SpecMeta {
            feature: "backtest-fast-cache".to_string(),
            domain: Some("05_backtest_analyser".to_string()),
            created_at: "2026-09-04T11:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: SpecMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, deserialized);
    }

    #[test]
    fn test_archived_feature_resolution() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        let domain_archived = root.join("projects/completed/05_backtest_analyser/2026-09-04_backtest-fast-cache");
        fs::create_dir_all(&domain_archived).unwrap();

        let meta = SpecMeta {
            feature: "backtest-fast-cache".to_string(),
            domain: Some("05_backtest_analyser".to_string()),
            created_at: "2026-09-04T11:00:00Z".to_string(),
        };
        meta.save_to_dir(&domain_archived).unwrap();

        // 1. Direct search by feature name in completed
        let found = SpecManager::find_archived_feature(&root.join("projects/completed"), "backtest-fast-cache");
        assert_eq!(found, Some(domain_archived.clone()));

        // 2. Flat timestamped completed (no domain)
        let flat_archived = root.join("projects/completed/2026-09-04_flat-feature");
        fs::create_dir_all(&flat_archived).unwrap();
        let found_flat = SpecManager::find_archived_feature(&root.join("projects/completed"), "flat-feature");
        assert_eq!(found_flat, Some(flat_archived));
    }
}
