use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_mcp_discover_handshake() {
    let bin_path = env!("CARGO_BIN_EXE_deliver-cli");
    let mut child = Command::new(bin_path)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start deliver-cli mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    let request = r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}"#;
    writeln!(stdin, "{}", request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    let mut response_line = String::new();
    reader.read_line(&mut response_line).expect("Failed to read response");

    let _ = child.kill();
    let _ = child.wait();

    let val: serde_json::Value = serde_json::from_str(&response_line)
        .unwrap_or_else(|e| panic!("Failed to parse response JSON: {}. Response was: {:?}", e, response_line));

    assert_eq!(val["id"], 1);
    assert_eq!(val["jsonrpc"], "2.0");
    assert_eq!(val["result"]["serverInfo"]["name"], "deliver-cli");
}
