use async_trait::async_trait;
use crate::manager::spec_manager::SpecManager;
use crate::io::openapi_loader::OpenApiLoader;
use mcp_sdk_rs::{
    server::{Server, ServerHandler},
    transport::stdio::StdioTransport,
    types::{ClientCapabilities, Implementation, ServerCapabilities, Tool, ToolSchema},
    error::ErrorCode,
};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct DeliverServerHandler {
    pub manager: SpecManager,
    pub loader: OpenApiLoader,
    pub base_dir: std::path::PathBuf,
}

impl DeliverServerHandler {
    pub async fn run_stdio(self) -> Result<()> {
        let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel(100);
        let (stdout_tx, mut stdout_rx) = tokio::sync::mpsc::channel(100);

        let transport = Arc::new(StdioTransport::new(stdin_rx, stdout_tx));
        let handler = Arc::new(self);
        let server = Server::new(transport, handler);

        // Spawn task to read from stdin
        tokio::spawn(async move {
            let mut reader = BufReader::new(tokio::io::stdin());
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 { break; }
                let _ = stdin_tx.send(line.trim().to_string()).await;
                line.clear();
            }
        });

        // Spawn task to write to stdout
        tokio::spawn(async move {
            let mut stdout = tokio::io::stdout();
            while let Some(message) = stdout_rx.recv().await {
                let _ = stdout.write_all(message.as_bytes()).await;
                let _ = stdout.write_all(b"\n").await;
                let _ = stdout.flush().await;
            }
        });

        server.start().await.map_err(|e| anyhow!("Server error: {}", e))?;
        Ok(())
    }
}

#[async_trait]
impl ServerHandler for DeliverServerHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, mcp_sdk_rs::Error> {
        Ok(ServerCapabilities {
            tools: Some(json!({ "listChanged": false })),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), mcp_sdk_rs::Error> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, mcp_sdk_rs::Error> {
        match method {
            "tools/list" => {
                let tools = vec![
                    Tool {
                        name: "sc_status".to_string(),
                        description: "Get current workflow status".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "feature": { "type": "string" }
                            })),
                            required: None,
                        }),
                        annotations: None,
                    },
                    Tool {
                        name: "sc_init".to_string(),
                        description: "Initialize a new feature".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "name": { "type": "string" },
                                "description": { "type": "string" },
                                "mode": { "type": "string" }
                            })),
                            required: None,
                        }),
                        annotations: None,
                    },
                    Tool {
                        name: "sc_plan".to_string(),
                        description: "Advance workflow phase".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "feature": { "type": "string" },
                                "instruction": { "type": "string" }
                            })),
                            required: None,
                        }),
                        annotations: None,
                    },
                    Tool {
                        name: "sc_approve".to_string(),
                        description: "Approve current phase".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "feature": { "type": "string" }
                            })),
                            required: None,
                        }),
                        annotations: None,
                    },
                    Tool {
                        name: "sc_todo_complete".to_string(),
                        description: "Complete a task".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "id": { "type": "string" },
                                "feature": { "type": "string" }
                            })),
                            required: Some(vec!["id".to_string()]),
                        }),
                        annotations: None,
                    },
                ];
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let params = params.ok_or_else(|| mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing params"))?;
                let name = params.get("name").and_then(|v| v.as_str()).ok_or_else(|| mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing tool name"))?;
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let result = match name {
                    "sc_status" => {
                        let feature = args.get("feature").and_then(|v| v.as_str());
                        self.manager.get_status_summary(&self.base_dir, feature, &self.loader)
                    }
                    "sc_init" => {
                        let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let desc = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let mode = args.get("mode").and_then(|v| v.as_str()).map(|s| s.to_string());
                        self.manager.init(&self.base_dir, name, desc, mode, &self.loader)
                            .map_err(|e| mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string()))?
                    }
                    "sc_plan" => {
                        let feature = args.get("feature").and_then(|v| v.as_str());
                        let ins = args.get("instruction").and_then(|v| v.as_str());
                        self.manager.plan(&self.base_dir, feature, ins, &self.loader)
                            .map_err(|e| mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string()))?
                    }
                    "sc_approve" => {
                        let feature = args.get("feature").and_then(|v| v.as_str());
                        self.manager.approve(&self.base_dir, feature, &self.loader)
                            .map_err(|e| mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string()))?
                    }
                    "sc_todo_complete" => {
                        let id = args.get("id").and_then(|v| v.as_str()).ok_or_else(|| mcp_sdk_rs::Error::protocol(ErrorCode::InvalidParams, "Missing task id".to_string()))?;
                        let feature = args.get("feature").and_then(|v| v.as_str());
                        self.manager.complete_task(&self.base_dir, feature, id, &self.loader)
                            .map_err(|e| mcp_sdk_rs::Error::protocol(ErrorCode::InternalError, e.to_string()))?
                    }
                    _ => return Err(mcp_sdk_rs::Error::protocol(ErrorCode::MethodNotFound, format!("Tool {} not found", name))),
                };

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": result
                        }
                    ]
                }))
            }
            _ => Err(mcp_sdk_rs::Error::protocol(ErrorCode::MethodNotFound, format!("Method {} not found", method))),
        }
    }
}
