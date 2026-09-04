use std::process::Command;
use tempfile::tempdir;
use std::fs;

#[test]
fn test_init_missing_name_failure() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    
    // Create a .spec_root to ensure we stay within the temp dir
    fs::write(root.join(".spec_root"), "").unwrap();
    
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    
    let output = Command::new(bin_path)
        .args(&["init"])
        .current_dir(&root)
        .output()
        .expect("Failed to run init");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("the following required arguments were not provided"));
    assert!(stderr.contains("--name <NAME>"));
}

#[test]
fn test_init_flag_success() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();
    
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    
    let output = Command::new(bin_path)
        .args(&["init", "--name", "Correct-Name"])
        .current_dir(&root)
        .output()
        .expect("Failed to run init");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Feature initialized"));
    
    assert!(root.join("projects/active/Correct-Name/Specification.md").exists());
}

#[test]
fn test_init_with_domain() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");

    let output = Command::new(bin_path)
        .args(&[
            "init",
            "--name",
            "backtest-fast-cache",
            "--domain",
            "05_backtest_analyser_and_scenarios",
        ])
        .current_dir(&root)
        .output()
        .expect("Failed to run init");

    assert!(output.status.success(), "Init failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Feature initialized"));
    assert!(stdout.contains("domain: 05_backtest_analyser_and_scenarios"));

    let feature_dir = root.join("projects/active/backtest-fast-cache");
    let meta_file = feature_dir.join(".spec-meta.json");
    assert!(meta_file.exists(), ".spec-meta.json must be created");

    let meta_content = fs::read_to_string(meta_file).unwrap();
    assert!(meta_content.contains("\"feature\": \"backtest-fast-cache\""));
    assert!(meta_content.contains("\"domain\": \"05_backtest_analyser_and_scenarios\""));
    assert!(meta_content.contains("\"created_at\""));

    let epoch_content = fs::read_to_string(feature_dir.join(".epoch-context.md")).unwrap();
    assert!(epoch_content.contains("**Domain:** 05_backtest_analyser_and_scenarios"));
}


