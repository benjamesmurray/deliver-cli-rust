use std::process::Command;
use tempfile::tempdir;
use std::fs;

#[test]
fn test_init_positional_failure() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    
    // Create a .spec_root to ensure we stay within the temp dir
    fs::write(root.join(".spec_root"), "").unwrap();
    
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    
    let output = Command::new(bin_path)
        .args(&["init", "Broken Name"])
        .current_dir(&root)
        .output()
        .expect("Failed to run init");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No project name detected"));
    assert!(stderr.contains("init' requires the '--name' flag"));
    assert!(stderr.contains("--name \"Broken Name\""));
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
    assert!(stdout.contains("Created new specification template"));
    
    assert!(root.join("projects/active/Correct-Name/Specification.md").exists());
}

#[test]
fn test_init_no_args_fallback() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::write(root.join(".spec_root"), "").unwrap();
    
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    
    let output = Command::new(bin_path)
        .args(&["init"])
        .current_dir(&root)
        .output()
        .expect("Failed to run init");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created new specification template"));
    assert!(stdout.contains("feature-"));
}
