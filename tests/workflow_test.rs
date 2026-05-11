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
    assert!(stdout.contains("Created new specification template"));
    
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
    let tasks_file_path = feature_path.join("Tasks.md");
    assert!(tasks_file_path.exists(), "Tasks.md should be auto-scaffolded after approve");

    // 6. Edit tasks (remove template tags)
    fs::write(&tasks_file_path, "# Tasks\n\n- [ ] **TSK-001**: Task 1").unwrap();

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
}
