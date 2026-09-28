use std::fs;
use std::process::Command;
use tempfile::tempdir;
use std::path::PathBuf;

#[test]
fn test_workflow_lifecycle() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    
    fs::write(root.join(".spec_root"), "").unwrap();
    println!("Root in test: {:?}", root);
    
    let feature_dir = root.join("my-feature");
    fs::create_dir(&feature_dir).unwrap();
    println!("Feature dir in test: {:?}", feature_dir);
    
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/assets/default-spec.yaml").canonicalize().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    
    // 1. Init
    let output = Command::new(bin_path)
        .args(&["init", "--name", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&feature_dir)
        .output()
        .expect("Failed to run init");
    
    assert!(output.status.success(), "Init failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Feature initialized"));
    
    let feature_path = root.join("projects/active/test-feature");
    let spec_file_path = feature_path.join("Specification.md");
    assert!(spec_file_path.exists());
    
    // 2. Status check
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("phase: specification"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("status: drafting"));

    // 3. Edit specification (remove template tags)
    fs::write(&spec_file_path, "# Test Feature\n\nSome specification here.").unwrap();
    
    // 4. Status check again
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(String::from_utf8_lossy(&output.stdout).contains("status: reviewing"));

    // 5. Approve (Should auto-scaffold tasks)
    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run approve");
    assert!(output.status.success(), "Approve failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✅ Phase \"Specification Document\" approved. You are now in the TASKS phase."));
    assert!(stdout.contains("NEXT STEPS:"));
    assert!(stdout.contains("Requirement: EACH task MUST have 'id' (numeric), 'title', 'description', and 'status'."));
    
    let tasks_file_path = feature_path.join("Tasks.json");
    assert!(tasks_file_path.exists(), "Tasks.json should be auto-scaffolded after approve");

    // 6. Edit tasks (set template_tags_present to false)
    let tasks_json = r#"{
        "template_tags_present": false,
        "tasks": [
            {
                "id": "1.1",
                "title": "Foundation & Setup",
                "description": "Establish the base environment and common definitions.",
                "status": "pending",
                "dependencies": []
            }
        ]
    }"#;
    fs::write(&tasks_file_path, tasks_json).unwrap();

    // Verify status is reviewing
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(String::from_utf8_lossy(&output.stdout).contains("status: reviewing"), "Should be reviewing with false flag");

    // 6b. Test with flag REMOVED
    let tasks_json_no_flag = r#"{
        "tasks": [
            {
                "id": "1.1",
                "title": "Foundation & Setup",
                "description": "Establish the base environment and common definitions.",
                "status": "pending",
                "dependencies": []
            }
        ]
    }"#;
    fs::write(&tasks_file_path, tasks_json_no_flag).unwrap();

    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(String::from_utf8_lossy(&output.stdout).contains("status: reviewing"), "Should be reviewing with flag REMOVED");

    // 7. Approve Tasks
    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run approve tasks");
    assert!(output.status.success(), "Approve tasks failed: {}", String::from_utf8_lossy(&output.stderr));

    // 8. Plan (Now that tasks are approved, it should show instructions)
    let output = Command::new(bin_path)
        .args(&["plan", "--feature", "test-feature", "--instruction", "Test instruction"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run plan");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Plan failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Test instruction"));

    // 9. Start Task
    let output = Command::new(bin_path)
        .args(&["todo", "start", "--id", "1.1", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run todo start");
    assert!(output.status.success(), "Todo start failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let tasks_content = fs::read_to_string(&tasks_file_path).unwrap();
    assert!(tasks_content.contains("\"status\": \"in_progress\""));

    // 10. Complete Task
    let output = Command::new(bin_path)
        .args(&["todo", "complete", "--id", "1.1", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run todo complete");
    assert!(output.status.success(), "Todo complete failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let tasks_content = fs::read_to_string(&tasks_file_path).unwrap();
    assert!(tasks_content.contains("\"status\": \"completed\""));

    // 11. Plan (All tasks complete -> should archive with date prefix)
    let output = Command::new(bin_path)
        .args(&["plan", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run plan for archival");
    assert!(output.status.success(), "Archival plan failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Workflow is completely finished"));
    assert!(stdout.contains("Successfully archived project to"));

    // Check that projects/completed/<YYYY-MM-DD>_test-feature exists
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let expected_archive = root.join("projects/completed").join(format!("{}_test-feature", today));
    assert!(expected_archive.exists(), "Expected archive dir {:?} to exist", expected_archive);

    // 12. Status check on archived feature by original name
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status on archived feature");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase: completed"));
    assert!(stdout.contains("status: archived"));
}

#[test]
fn test_workflow_domain_archival() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    // Create .spec.json with domain configuration
    let spec_config = r#"{
        "completed_directory": "projects/completed",
        "archive_format": "{domain}/{date}_{feature}",
        "domains": [
            "05_backtest_analyser_and_scenarios",
            "06_execution_strategies"
        ]
    }"#;
    fs::write(root.join(".spec.json"), spec_config).unwrap();

    let spec_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/assets/default-spec.yaml").canonicalize().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");

    // 1. Init with domain
    let output = Command::new(bin_path)
        .args(&[
            "init",
            "--name",
            "fast-cache",
            "--domain",
            "05_backtest_analyser_and_scenarios",
        ])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run init");
    assert!(output.status.success());

    let feature_path = root.join("projects/active/fast-cache");
    let spec_file = feature_path.join("Specification.md");

    // 2. Edit spec
    fs::write(&spec_file, "# Fast Cache\n\nSpecification details.").unwrap();

    // 3. Approve spec
    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "fast-cache"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run approve");
    assert!(output.status.success());

    // 4. Edit tasks
    let tasks_json = r#"{
        "template_tags_present": false,
        "tasks": [
            {
                "id": "1.1",
                "title": "Build Cache",
                "description": "Implement caching",
                "status": "completed",
                "dependencies": []
            }
        ]
    }"#;
    fs::write(feature_path.join("Tasks.json"), tasks_json).unwrap();

    // 5. Approve tasks
    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "fast-cache"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run approve tasks");
    assert!(output.status.success());

    // 6. Plan to archive
    let output = Command::new(bin_path)
        .args(&["plan", "--feature", "fast-cache"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run plan to archive");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Workflow is completely finished"));

    // 7. Verify archived location: projects/completed/05_backtest_analyser_and_scenarios/<date>_fast-cache
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let expected_path = root
        .join("projects/completed")
        .join("05_backtest_analyser_and_scenarios")
        .join(format!("{}_fast-cache", today));
    assert!(expected_path.exists(), "Expected domain-partitioned archive at {:?}", expected_path);

    // 8. Verify status can still resolve the archived feature by name
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "fast-cache"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase: completed"));
    assert!(stdout.contains("status: archived"));
    assert!(stdout.contains("domain: 05_backtest_analyser_and_scenarios"));
}

#[test]
fn test_archive_prunes_empty_parent_directories() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    let spec_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/assets/default-spec.yaml").canonicalize().unwrap();

    // Create a nested feature in projects/active/06_domain/my-nested-feature
    let nested_domain = root.join("projects/active/06_domain");
    let feature_dir = nested_domain.join("my-nested-feature");
    fs::create_dir_all(&feature_dir).unwrap();

    // Scaffold valid completed Tasks.json & Specification.md
    fs::write(feature_dir.join("Specification.md"), "# Spec\nValid spec").unwrap();
    let tasks_json = r#"{
        "template_tags_present": false,
        "tasks": [
            {
                "id": "1.1",
                "title": "Task 1",
                "description": "Done",
                "status": "completed",
                "dependencies": []
            }
        ]
    }"#;
    fs::write(feature_dir.join("Tasks.json"), tasks_json).unwrap();

    // Run archive via CLI
    let output = Command::new(bin_path)
        .args(&["archive", "--feature", "06_domain/my-nested-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run archive");
    assert!(output.status.success(), "Archive failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify parent directory 06_domain was pruned
    assert!(!nested_domain.exists(), "Empty parent domain directory should have been pruned");
    // Verify projects/active root still exists
    assert!(root.join("projects/active").exists(), "projects/active should still exist");
}

#[test]
fn test_clean_empty_nested_active_directories() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");

    let empty_deep_dir = root.join("projects/active/empty_domain/empty_sub");
    fs::create_dir_all(&empty_deep_dir).unwrap();

    let occupied_dir = root.join("projects/active/active_domain/feature");
    fs::create_dir_all(&occupied_dir).unwrap();
    fs::write(occupied_dir.join("dummy.txt"), "hello").unwrap();

    // Run clean via CLI
    let output = Command::new(bin_path)
        .arg("clean")
        .current_dir(&root)
        .output()
        .expect("Failed to run clean");
    assert!(output.status.success(), "Clean failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cleaned 2 empty director(ies)"));

    assert!(!root.join("projects/active/empty_domain").exists());
    assert!(root.join("projects/active/active_domain/feature").exists());
}

#[test]
fn test_abort_active_feature_and_prunes_parents() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    let nested_domain = root.join("projects/active/temp_domain");
    let feature_dir = nested_domain.join("abandoned-feature");
    fs::create_dir_all(&feature_dir).unwrap();
    fs::write(feature_dir.join("Specification.md"), "abandoned").unwrap();

    // Set .spec_last_used to this feature
    fs::write(root.join(".spec_last_used"), "projects/active/temp_domain/abandoned-feature").unwrap();

    // Run abort via CLI
    let output = Command::new(bin_path)
        .args(&["abort", "--feature", "temp_domain/abandoned-feature"])
        .current_dir(&root)
        .output()
        .expect("Failed to run abort");
    assert!(output.status.success(), "Abort failed: {}", String::from_utf8_lossy(&output.stderr));

    // Feature should be gone
    assert!(!feature_dir.exists());
    // Empty parent domain should be cleaned
    assert!(!nested_domain.exists());
    // .spec_last_used pointing to it should be removed
    assert!(!root.join(".spec_last_used").exists());
}

#[test]
fn test_abort_fails_for_non_active_feature() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    let completed_feature = root.join("projects/completed/finished-feature");
    fs::create_dir_all(&completed_feature).unwrap();
    fs::write(completed_feature.join("Specification.md"), "done").unwrap();

    let output = Command::new(bin_path)
        .args(&["abort", "--feature", "projects/completed/finished-feature"])
        .current_dir(&root)
        .output()
        .expect("Failed to run abort");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot abort a feature that is not in projects/active"));
}

#[test]
fn test_domain_cli_set_and_auto_discovery() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    let spec_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/assets/default-spec.yaml").canonicalize().unwrap();

    // 1. Create simulated completed domain directories (NO .spec.json!)
    let d1 = root.join("projects/completed/05_backtest_analyser/dummy");
    let d2 = root.join("projects/completed/06_execution_strategies/dummy");
    fs::create_dir_all(&d1).unwrap();
    fs::create_dir_all(&d2).unwrap();

    // 2. Test domain list auto-discovery
    let output = Command::new(bin_path)
        .args(&["domain", "list"])
        .current_dir(&root)
        .output()
        .expect("Failed to run domain list");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("05_backtest_analyser"));
    assert!(stdout.contains("06_execution_strategies"));

    // 3. Init feature without domain
    let output = Command::new(bin_path)
        .args(&["init", "--name", "post-domain-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run init");
    assert!(output.status.success());

    // 4. Set domain post-init
    let output = Command::new(bin_path)
        .args(&["domain", "set", "05_backtest_analyser", "--feature", "post-domain-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run domain set");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Domain set to '05_backtest_analyser'"));

    // 5. Get domain
    let output = Command::new(bin_path)
        .args(&["domain", "get", "--feature", "post-domain-feature"])
        .current_dir(&root)
        .output()
        .expect("Failed to run domain get");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Domain: 05_backtest_analyser"));

    // 6. Verify .epoch-context.md was updated
    let epoch_path = root.join("projects/active/post-domain-feature/.epoch-context.md");
    let epoch_content = fs::read_to_string(&epoch_path).unwrap();
    assert!(epoch_content.contains("**Domain:** 05_backtest_analyser"));

    // 7. Advance and archive feature
    let spec_file = root.join("projects/active/post-domain-feature/Specification.md");
    fs::write(&spec_file, "# Post Domain Spec\nNo template tags here.").unwrap();

    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "post-domain-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to approve spec");
    assert!(output.status.success());

    let tasks_json = r#"{
        "template_tags_present": false,
        "tasks": [
            {
                "id": "1",
                "title": "Build Post Domain",
                "description": "Done",
                "status": "completed",
                "dependencies": []
            }
        ]
    }"#;
    fs::write(root.join("projects/active/post-domain-feature/Tasks.json"), tasks_json).unwrap();

    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "post-domain-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to approve tasks");
    assert!(output.status.success());

    // Plan -> archive
    let output = Command::new(bin_path)
        .args(&["plan", "--feature", "post-domain-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run plan");
    assert!(output.status.success());

    // Verify archived into auto-discovered domain directory
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let expected_archive = root
        .join("projects/completed")
        .join("05_backtest_analyser")
        .join(format!("{}_post-domain-feature", today));
    assert!(expected_archive.exists(), "Expected archive at {:?}", expected_archive);
}
