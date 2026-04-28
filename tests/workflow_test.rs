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
    
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("legacy_deliver/api/spec-workflow.openapi.yaml").canonicalize().unwrap();
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
    assert!(stdout.contains("Created new requirements template"));
    
    let feature_path = root.join("projects/active/test-feature");
    let req_path = feature_path.join("Requirements.md");
    assert!(req_path.exists());
    
    // 2. Status check
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("phase: requirements"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("status: drafting"));

    // 3. Edit requirements (remove template tags)
    fs::write(&req_path, "# Test Feature\n\nSome requirements here.").unwrap();
    
    // 4. Status check again
    let output = Command::new(bin_path)
        .args(&["status", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run status");
    assert!(String::from_utf8_lossy(&output.stdout).contains("status: reviewing"));

    // 5. Approve
    let output = Command::new(bin_path)
        .args(&["approve", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run approve");
    assert!(output.status.success(), "Approve failed: {}", String::from_utf8_lossy(&output.stderr));

    // 6. Plan (Scaffold design)
    let output = Command::new(bin_path)
        .args(&["plan", "--feature", "test-feature"])
        .env("SPEC_PATH", &spec_path)
        .current_dir(&root)
        .output()
        .expect("Failed to run plan");
    assert!(output.status.success(), "Plan failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(feature_path.join("Design.md").exists());
}
