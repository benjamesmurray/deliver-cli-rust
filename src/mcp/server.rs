use async_trait::async_trait;
use crate::manager::spec_manager::SpecManager;
use crate::io::openapi_loader::OpenApiLoader;
use rust_mcp_sdk::{
    *,
    mcp_server::{server_runtime, ServerHandler, ToMcpServerHandler, McpServerOptions},
    schema::*,
    macros,
};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[macros::mcp_tool(name = "sc_status", description = "Get current workflow status")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScStatus {
    pub feature: Option<String>,
}

#[macros::mcp_tool(name = "sc_init", description = "Initialize a new feature")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScInit {
    pub name: Option<String>,
    pub description: Option<String>,
    pub mode: Option<String>,
}

#[macros::mcp_tool(name = "sc_plan", description = "Advance workflow phase")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScPlan {
    pub feature: Option<String>,
    pub instruction: Option<String>,
}

#[macros::mcp_tool(name = "sc_approve", description = "Approve current phase")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScApprove {
    pub feature: Option<String>,
}

#[macros::mcp_tool(name = "sc_todo_complete", description = "Complete a task")]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ScTodoComplete {
    pub id: String,
    pub feature: Option<String>,
}

pub struct DeliverServerHandler {
    pub manager: SpecManager,
    pub loader: OpenApiLoader,
    pub base_dir: std::path::PathBuf,
}

impl DeliverServerHandler {
    pub async fn run_stdio(self) -> Result<()> {
        let server_info = InitializeResult {
            server_info: Implementation {
                name: "deliver-cli".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: Some("Streamlined MCP server for managing spec workflow".into()),
                icons: vec![],
                title: Some("Deliver CLI MCP Server".into()),
                website_url: None,
            },
            capabilities: ServerCapabilities {
                tools: Some(ServerCapabilitiesTools {
                    list_changed: None,
                }),
                ..Default::default()
            },
            protocol_version: ProtocolVersion::V2025_11_25.into(),
            instructions: None,
            meta: None,
        };

        let transport = StdioTransport::new(TransportOptions::default())
            .map_err(|e| anyhow!("Failed to create transport: {}", e))?;
        
        let handler = self.to_mcp_server_handler();
        let options = McpServerOptions {
            server_details: server_info,
            transport,
            handler,
            task_store: None,
            client_task_store: None,
            message_observer: None,
        };
        let server = server_runtime::create_server(options);
        
        server.start().await.map_err(|e| anyhow!("Server error: {}", e))?;
        Ok(())
    }
}

#[async_trait]
impl ServerHandler for DeliverServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: vec![
                ScStatus::tool(),
                ScInit::tool(),
                ScPlan::tool(),
                ScApprove::tool(),
                ScTodoComplete::tool(),
            ],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_name = params.name.clone();
        let result = match params.name.as_str() {
            "sc_status" => {
                let args: ScStatus = serde_json::from_value(serde_json::Value::Object(params.arguments.unwrap_or_default()))
                    .map_err(|e| CallToolError::invalid_arguments(tool_name, Some(e.to_string())))?;
                self.manager.get_status_summary(&self.base_dir, args.feature.as_deref(), &self.loader)
            }
            "sc_init" => {
                let args: ScInit = serde_json::from_value(serde_json::Value::Object(params.arguments.unwrap_or_default()))
                    .map_err(|e| CallToolError::invalid_arguments(tool_name, Some(e.to_string())))?;
                self.manager.init(&self.base_dir, args.name, args.description, args.mode, &self.loader)
                    .map_err(|e| CallToolError::from_message(e.to_string()))?
            }
            "sc_plan" => {
                let args: ScPlan = serde_json::from_value(serde_json::Value::Object(params.arguments.unwrap_or_default()))
                    .map_err(|e| CallToolError::invalid_arguments(tool_name, Some(e.to_string())))?;
                self.manager.plan(&self.base_dir, args.feature.as_deref(), args.instruction.as_deref(), &self.loader)
                    .map_err(|e| CallToolError::from_message(e.to_string()))?
            }
            "sc_approve" => {
                let args: ScApprove = serde_json::from_value(serde_json::Value::Object(params.arguments.unwrap_or_default()))
                    .map_err(|e| CallToolError::invalid_arguments(tool_name, Some(e.to_string())))?;
                self.manager.approve(&self.base_dir, args.feature.as_deref(), &self.loader)
                    .map_err(|e| CallToolError::from_message(e.to_string()))?
            }
            "sc_todo_complete" => {
                let args: ScTodoComplete = serde_json::from_value(serde_json::Value::Object(params.arguments.unwrap_or_default()))
                    .map_err(|e| CallToolError::invalid_arguments(tool_name, Some(e.to_string())))?;
                self.manager.complete_task(&self.base_dir, args.feature.as_deref(), &args.id, &self.loader)
                    .map_err(|e| CallToolError::from_message(e.to_string()))?
            }
            _ => return Err(CallToolError::unknown_tool(params.name)),
        };

        Ok(CallToolResult::text_content(vec![result.into()]))
    }
}
