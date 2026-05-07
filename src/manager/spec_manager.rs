use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;
use crate::markdown::analyzer::MarkdownAnalyzer;
use crate::io::openapi_loader::OpenApiLoader;
use anyhow::{Result, anyhow};

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
    const LAST_USED_FILE: &'static str = ".spec_last_used";

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
            Err(e) => return format!("spec_status:\n  phase: error\n  error: {}\n  next_step: use mcpx with server=\"spec\" tool=\"sc_init\" and name=\"your-feature\"", e),
        };

        let state = Self::get_workflow_state(&feature_path, loader);
        let mode = Self::get_mode(&feature_path);
        let root_dir = Self::find_project_root(base_dir);

        let is_archived = feature_path.starts_with(root_dir.join("projects/completed")) || 
                         feature_path.starts_with(root_dir.join("completed"));

        let mut all_tasks_complete = false;
        if state.tasks.exists && state.tasks.edited {
            let tasks_file = loader.get_file_name("tasks").cloned().unwrap_or("tasks.md".to_string());
            if let Ok(content) = fs::read_to_string(feature_path.join(tasks_file)) {
                let tasks = crate::markdown::parser::TaskParser::parse(&content);
                if !tasks.is_empty() {
                    all_tasks_complete = Self::are_all_tasks_done(&tasks);
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
            next_steps = "use mcpx with server=\"spec\" and tool=\"sc_init\" to initialize specification.".to_string();
        } else if !state.specification.edited {
            phase = "specification";
            status = "drafting";
            blockers.push("template_tags_present");
            next_steps = "Write specification.md and use mcpx with server=\"spec\" and tool=\"sc_plan\" to advance.".to_string();
        } else if !state.specification.approved {
            phase = "specification";
            status = "reviewing";
            if mode == "one-shot" {
                next_steps = "Resolve ambiguities then use mcpx with server=\"spec\" and tool=\"sc_plan\".".to_string();
            } else {
                next_steps = "Review and use mcpx with server=\"spec\" and tool=\"sc_approve\".".to_string();
            }
        } else if !state.tasks.exists {
            phase = "specification";
            status = "approved";
            next_steps = "use mcpx with server=\"spec\" and tool=\"sc_plan\" to scaffold tasks.".to_string();
        } else if !state.tasks.edited {
            phase = "tasks";
            status = "drafting";
            blockers.push("template_tags_present");
            next_steps = "Write tasks.md and use mcpx with server=\"spec\" and tool=\"sc_todo_start\" to begin.".to_string();
        } else if !state.tasks.approved {
            phase = "tasks";
            status = "reviewing";
            if mode == "one-shot" {
                next_steps = "Resolve ambiguities then use mcpx with server=\"spec\" and tool=\"sc_todo_start\".".to_string();
            } else {
                next_steps = "Review and use mcpx with server=\"spec\" and tool=\"sc_approve\".".to_string();
            }
        } else if !all_tasks_complete {
            phase = "implementation";
            status = "active";
            next_steps = "Proceed with tasks using mcpx with server=\"spec\" and tool=\"sc_todo_start\".".to_string();
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

        format!(
            "spec_status:\n  feature: {}\n  phase: {}\n  status: {}\n  next_step: {}\n  blockers: [{}]\n  mode: {}{}",
            rel_feature, phase, status, next_steps, blockers.join(", "), mode, epoch_info
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
                        return Err(anyhow!("Cannot advance while there are active open questions in the epoch context. Please resolve them using `use mcpx with server=\"spec\" tool=\"sc_epoch\" and openQuestions=\"None\"`."));
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
                    return Err(anyhow!("Approval blocked: Recent feedback was recorded. To prevent misinterpretation of information as approval, you must wait for a separate turn and explicit user approval before calling mcpx with server=\"spec\" and tool=\"sc_approve\"."));
                }
            }
            fs::remove_file(feedback_marker)?;
        }

        let approved_path = feature_path.join(format!(".spec-{}-approved", phase));
        fs::write(approved_path, chrono::Utc::now().to_rfc3339())?;
        
        let display_name = loader.spec.global_config.stage_names.get(&phase).cloned().unwrap_or(phase);
        Ok(format!("✅ Phase \"{}\" approved. use mcpx with server=\"spec\" and tool=\"sc_plan\" to scaffold next phase.", display_name))
    }

    pub fn archive(&self, base_dir: &Path, feature_name: Option<&str>) -> Result<String> {
        let root_dir = Self::find_project_root(base_dir);
        let current_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let target_dir = root_dir.join("projects/completed");
        
        if current_path.starts_with(&target_dir) {
            return Ok("Project is already in the completed directory.".to_string());
        }

        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)?;
        }

        let feature_dir_name = current_path.file_name().ok_or_else(|| anyhow!("Invalid feature path"))?;
        let target_path = target_dir.join(feature_dir_name);

        match fs::rename(&current_path, &target_path) {
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::Other => {
                fs_extra::dir::move_dir(&current_path, &target_path, &fs_extra::dir::CopyOptions::new())?;
            }
            Err(e) => return Err(e.into()),
        }

        let rel_target = target_path.strip_prefix(&root_dir).unwrap_or(&target_path);
        fs::write(root_dir.join(Self::LAST_USED_FILE), rel_target.to_string_lossy().as_bytes())?;
        Ok(format!("Successfully archived project to {}.", rel_target.to_string_lossy()))
    }

    pub fn start_task(&self, base_dir: &Path, feature_name: Option<&str>, task_id: &str, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let tasks_file = loader.get_file_name("tasks").cloned().unwrap_or("tasks.md".to_string());
        let tasks_path = feature_path.join(tasks_file);

        if !tasks_path.exists() {
            return Err(anyhow!("tasks.md file does not exist. Please complete writing the tasks document first."));
        }

        let content = fs::read_to_string(&tasks_path)?;
        let flat_tasks = crate::markdown::parser::TaskParser::parse_flat(&content);

        let _task = flat_tasks.iter().find(|t| t.id == task_id)
            .ok_or_else(|| anyhow!("Task {} not found", task_id))?;

        let updated_content = crate::markdown::updater::MarkdownTaskUpdater::update_task_status_char(&content, task_id, '/');
        fs::write(&tasks_path, updated_content)?;

        Ok(format!("🚀 Task {} marked as IN PROGRESS.", task_id))
    }

    pub fn complete_task(&self, base_dir: &Path, feature_name: Option<&str>, task_id: &str, loader: &OpenApiLoader) -> Result<String> {
        let feature_path = Self::resolve_feature_path(base_dir, feature_name)?;
        let tasks_file = loader.get_file_name("tasks").cloned().unwrap_or("tasks.md".to_string());
        let tasks_path = feature_path.join(tasks_file);

        if !tasks_path.exists() {
            return Err(anyhow!("tasks.md file does not exist. Please complete writing the tasks document first."));
        }

        let content = fs::read_to_string(&tasks_path)?;
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

        // Clear feedback marker if it exists
        let feedback_marker = feature_path.join(".spec-last-feedback");
        if feedback_marker.exists() {
            fs::remove_file(feedback_marker)?;
        }

        Ok(format!("✅ Task {} marked as completed!", task_id))
    }
    pub fn init(&self, base_dir: &Path, name: Option<String>, description: Option<String>, mode: Option<String>, loader: &OpenApiLoader) -> Result<String> {
        let feature_name = name.unwrap_or_else(|| {
            format!("feature-{}", chrono::Utc::now().timestamp())
        });
        
        let feature_path = Self::resolve_feature_path(base_dir, Some(&feature_name))?;
        if !feature_path.exists() {
            fs::create_dir_all(&feature_path)?;
        }
        
        if let Some(m) = mode {
            Self::set_mode(&feature_path, &m)?;
        }

        let spec_file = loader.get_file_name("specification").cloned().unwrap_or("specification.md".to_string());
        let spec_path = feature_path.join(spec_file);
        
        let msg: String;
        if !spec_path.exists() {
            let template = loader.get_template("specification").ok_or_else(|| anyhow!("Specification template not found"))?;
            let mut vars = HashMap::new();
            vars.insert("featureName".to_string(), feature_name.clone());
            vars.insert("introduction".to_string(), description.unwrap_or_else(|| "Initial specification".to_string()));
            
            let content = crate::io::template_engine::TemplateEngine::interpolate(template, &vars);
            fs::write(&spec_path, content)?;
            fs::write(feature_path.join(".epoch-context.md"), "# Epoch Context\n\n**Current Phase:** Specification\n\n")?;
            msg = format!("✅ Created new specification template at: {:?}", spec_path);
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
             return self.init(base_dir, feature.map(|s| s.to_string()), None, None, loader);
        } else if !state.specification.edited {
            message = format!("Please finish editing {} (remove all <template> tags) before advancing.", loader.get_file_name("specification").unwrap());
        } else if !state.specification.approved && mode != "one-shot" {
            message = "Specification drafted but not yet approved. Please review and run `use mcpx with server=\"spec\" and tool=\"sc_approve\"` before advancing.".to_string();
        } else if !state.tasks.exists {
            if mode == "one-shot" {
                Self::validate_transition(&feature_path)?;
            }
            let template = loader.get_template("tasks").ok_or_else(|| anyhow!("Tasks template not found"))?;
            let mut vars = HashMap::new();
            vars.insert("featureName".to_string(), feature_path.file_name().unwrap().to_string_lossy().to_string());
            let mut content = crate::io::template_engine::TemplateEngine::interpolate(template, &vars);
            if let Some(ins) = instruction {
                content.push_str(&format!("\n\n> **Guidance:** {}", ins));
            }
            let tasks_file = loader.get_file_name("tasks").unwrap();
            fs::write(feature_path.join(tasks_file), content)?;
            fs::write(feature_path.join(".epoch-context.md"), "# Epoch Context\n\n**Current Phase:** Implementation Planning\n\n")?;
            message = format!("Specification complete. Scaffolding {}. Epoch context reset.", tasks_file);
        } else if !state.tasks.edited {
            message = format!("Please finish editing {} (remove all <template> tags) before advancing.", loader.get_file_name("tasks").unwrap());
        } else if !state.tasks.approved && mode != "one-shot" {
            message = "Tasks drafted but not yet approved. Please review and run `use mcpx with server=\"spec\" and tool=\"sc_approve\"` before advancing.".to_string();
        } else {
            let mut all_tasks_complete = false;
            let tasks_file = loader.get_file_name("tasks").unwrap();
            let tasks_path = feature_path.join(tasks_file);
            if tasks_path.exists() {
                let content = fs::read_to_string(tasks_path)?;
                let tasks = crate::markdown::parser::TaskParser::parse(&content);
                if !tasks.is_empty() {
                    all_tasks_complete = Self::are_all_tasks_done(&tasks);
                }
            }

            if !all_tasks_complete {
                let mut msg = "Not all implementation tasks are complete. Proceed with `mcpx with server=\"spec\" and tool=\"sc_todo_start\"` or finish tasks manually.".to_string();
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
