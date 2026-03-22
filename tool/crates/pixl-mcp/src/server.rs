use crate::{handlers, state::McpState, tools};
use rmcp::{
    Error as McpError,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content,
        InitializeResult, ListToolsResult, PaginatedRequestParams,
        ServerCapabilities, ServerInfo, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
};
use std::sync::Mutex;

pub struct PixlServer {
    state: Mutex<McpState>,
}

impl PixlServer {
    pub fn new() -> Self {
        PixlServer {
            state: Mutex::new(McpState::new()),
        }
    }

    pub fn with_source(source: &str) -> Result<Self, String> {
        let state = McpState::from_source(source)?;
        Ok(PixlServer {
            state: Mutex::new(state),
        })
    }
}

impl ServerHandler for PixlServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info.name = "pixl".to_string();
        info.server_info.version = "0.1.0".to_string();
        info.capabilities.tools = Some(ToolsCapability {
            list_changed: Some(false),
        });
        info
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let tool_list = tools::tool_definitions();
        let mut result = ListToolsResult::default();
        result.tools = tool_list;
        std::future::ready(Ok(result))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let name = request.name.to_string();
        let args = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let result = handlers::handle_tool(&self.state, &name, &args);
        let text = serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string());

        let mut content = vec![Content::text(text)];
        if let Some(b64) = result.get("preview_b64").and_then(|v| v.as_str()) {
            content.push(Content::image(b64.to_string(), "image/png".to_string()));
        }

        let is_error = result.get("error").is_some();
        let mut call_result = CallToolResult::default();
        call_result.content = content;
        call_result.is_error = Some(is_error);

        std::future::ready(Ok(call_result))
    }
}

pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let server = PixlServer::new();
    let transport = rmcp::transport::stdio();
    let service = rmcp::service::serve_server(server, transport).await?;
    service.waiting().await?;
    Ok(())
}

pub async fn run_stdio_with_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(path)?;
    let server = PixlServer::with_source(&source)
        .map_err(|e| format!("failed to load {}: {}", path, e))?;
    let transport = rmcp::transport::stdio();
    let service = rmcp::service::serve_server(server, transport).await?;
    service.waiting().await?;
    Ok(())
}
