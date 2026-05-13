mod manager;
mod workflow;
mod markdown;
mod io;
mod cli;
mod mcp;

use std::path::Path;
use clap::Parser;
use anyhow::{Result, anyhow};
use crate::cli::commands::{Cli, Commands, TodoAction};
use crate::manager::spec_manager::SpecManager;
use crate::io::openapi_loader::OpenApiLoader;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_dir = std::env::current_dir()?;
    
    // Path to OpenAPI spec - adjust as needed for distribution
    let spec_env = std::env::var("SPEC_PATH").unwrap_or_default();
    let loader = if !spec_env.is_empty() && Path::new(&spec_env).exists() {
        OpenApiLoader::load(Path::new(&spec_env)).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load OpenAPI spec from {}: {}. Falling back to default configuration.", spec_env, e);
            OpenApiLoader::load_default().expect("Failed to load embedded default config")
        })
    } else {
        OpenApiLoader::load_default().expect("Failed to load embedded default config")
    };
    let manager = SpecManager;

    match cli.command {
        Commands::Mcp => {
            let handler = crate::mcp::server::DeliverServerHandler {
                manager,
                loader,
                base_dir,
            };
            handler.run_stdio().await?;
        }
        Commands::Status { feature } => {
            let output = manager.get_status_summary(&base_dir, feature.as_deref(), &loader);
            println!("{}", output);
        }
        Commands::Verify { feature } => {
            let status = manager.get_status_summary(&base_dir, feature.as_deref(), &loader);
            println!("Project state verified.\n\n{}", status);
        }
        Commands::Init { name, description, mode, extraneous_args } => {
            if name.is_none() && !extraneous_args.is_empty() {
                return Err(anyhow!(
                    "No project name detected. It looks like you tried to name the project using a positional argument, but 'init' requires the '--name' flag.\n\nCorrect Syntax: deliver-cli init --name \"{}\"",
                    extraneous_args[0]
                ));
            }
            let output = manager.init(&base_dir, name, description, mode, &loader)?;
            println!("{}", output);
        }
        Commands::Approve { feature } => {
            let output = manager.approve(&base_dir, feature.as_deref(), &loader)?;
            println!("{}", output);
        }
        Commands::Archive { feature } => {
            let output = manager.archive(&base_dir, feature.as_deref())?;
            println!("{}", output);
        }
        Commands::Todo { action } => {
            match action {
                TodoAction::List { feature } => {
                    println!("{}", manager.get_status_summary(&base_dir, feature.as_deref(), &loader));
                }
                TodoAction::Start { id, feature } => {
                    let output = manager.start_task(&base_dir, feature.as_deref(), &id, &loader)?;
                    println!("{}", output);
                    println!("\n{}", manager.get_status_summary(&base_dir, feature.as_deref(), &loader));
                }
                TodoAction::Complete { id, feature } => {
                    let output = manager.complete_task(&base_dir, feature.as_deref(), &id, &loader)?;
                    println!("{}", output);
                    println!("\n{}", manager.get_status_summary(&base_dir, feature.as_deref(), &loader));
                }
            }
        }
        Commands::Epoch { feature, focus, intentions, hypotheses, open_questions } => {
            let feature_path = SpecManager::resolve_feature_path(&base_dir, feature.as_deref())?;
            let epoch_path = feature_path.join(".epoch-context.md");
            let mut epoch_content = if epoch_path.exists() {
                fs::read_to_string(&epoch_path)?
            } else {
                "# Epoch Context\n\n".to_string()
            };

            if let Some(f) = focus {
                Main::update_epoch_section(&mut epoch_content, "## Active Focus", &f);
            }
            if let Some(i) = intentions {
                Main::update_epoch_section(&mut epoch_content, "## Pending Intentions", &i);
            }
            if let Some(h) = hypotheses {
                Main::update_epoch_section(&mut epoch_content, "## Active Hypotheses", &h);
            }
            if let Some(q) = open_questions {
                Main::update_epoch_section(&mut epoch_content, "## Open Questions / Uncertainties", &q);
            }

            fs::write(epoch_path, epoch_content)?;
            println!("Epoch context updated successfully.");
            println!("\n{}", manager.get_status_summary(&base_dir, feature.as_deref(), &loader));
        }
        Commands::Mode { mode, feature } => {
            let feature_path = SpecManager::resolve_feature_path(&base_dir, feature.as_deref())?;
            SpecManager::set_mode(&feature_path, &mode)?;
            println!("Mode updated successfully to {}.", mode);
            println!("\n{}", manager.get_status_summary(&base_dir, feature.as_deref(), &loader));
        }
        Commands::Feedback { instruction, feature } => {
            let feature_path = SpecManager::resolve_feature_path(&base_dir, feature.as_deref())?;
            let epoch_path = feature_path.join(".epoch-context.md");
            if epoch_path.exists() {
                let mut content = fs::read_to_string(&epoch_path)?;
                let feedback_preview = if instruction.len() > 50 { format!("{}...", &instruction[..50]) } else { instruction.clone() };
                let new_section = format!("## Open Questions / Uncertainties\n*   None (Feedback received: {})\n\n", feedback_preview);
                
                let re = regex::Regex::new(r"(?s)## Open Questions / Uncertainties\n.*?(?:##|$)").unwrap();
                content = re.replace(&content, &new_section).to_string();
                fs::write(&epoch_path, content)?;
            }

            let feedback_marker = feature_path.join(".spec-last-feedback");
            fs::write(feedback_marker, chrono::Utc::now().to_rfc3339())?;
            
            println!("Feedback acknowledged and recorded. Open questions have been cleared.");
            println!("\n{}", manager.get_status_summary(&base_dir, feature.as_deref(), &loader));
        }
        Commands::Plan { feature, instruction } => {
            let output = manager.plan(&base_dir, feature.as_deref(), instruction.as_deref(), &loader)?;
            println!("{}", output);
        }
    }

    Ok(())
}

impl Main {
    fn update_epoch_section(content: &mut String, header: &str, value: &str) {
        let pattern = format!(r"(?s){}\n.*?(?:##|$)", regex::escape(header));
        let re = regex::Regex::new(&pattern).unwrap();
        let new_text = format!("{}\n*   {}\n\n", header, value);
        
        if re.is_match(content) {
            *content = re.replace(content, &new_text).to_string();
        } else {
            content.push_str(&new_text);
        }
    }
}

struct Main;
