use crate::{
    arguments::*,
    backend::{Backend, Reply},
};
use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};
use schemars::{JsonSchema, schema_for};
use serde_json::{Value, json};

pub struct Server {
    pub backend: Backend,
}
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions("Graph queries are read-only. Use discover_builds and explicit refresh_graph to analyze. Preserve expected_generation across related queries. Static relationships express certainty/provenance, not proof of runtime execution.")
    }
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: definitions(),
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        definitions().into_iter().find(|tool| tool.name == name)
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if self.get_tool(request.name.as_ref()).is_none() {
            return Err(ErrorData::method_not_found::<CallToolRequestMethod>());
        }
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        let reply = match request.name.as_ref() {
            "discover_builds" => match serde_json::from_value::<Empty>(arguments) {
                Ok(_) => self
                    .backend
                    .discovery(&context.ct)
                    .await
                    .map(|value| Reply {
                        value,
                        is_error: false,
                    }),
                Err(error) => return Ok(failure("invalid_arguments", error).into()),
            },
            "refresh_graph" => match serde_json::from_value::<RefreshArgs>(arguments) {
                Ok(args) => self.backend.refresh(args, &context.ct).await,
                Err(error) => return Ok(failure("invalid_arguments", error).into()),
            },
            name => match crate::arguments::parse(name, arguments) {
                Ok(query) => self.backend.query(query, &context.ct).await,
                Err(error) => return Ok(failure("invalid_arguments", error).into()),
            },
        };
        Ok(match reply {
            Ok(reply) => result(reply),
            Err(error) => failure("cli_transport_error", format!("{error:#}")),
        }
        .into())
    }
}
fn result(reply: Reply) -> CallToolResult {
    let summary = if reply.is_error {
        format!(
            "CLI error {}: {}",
            reply.value["error"]["code"].as_str().unwrap_or("unknown"),
            reply.value["error"]["message"]
                .as_str()
                .unwrap_or("CLI operation failed")
        )
    } else {
        "CLI operation completed; structuredContent contains the full response and metadata.".into()
    };
    let mut result = CallToolResult::structured(reply.value);
    result.content = vec![ContentBlock::text(summary)];
    result.is_error = Some(reply.is_error);
    result
}
fn failure(code: &str, message: impl std::fmt::Display) -> CallToolResult {
    result(Reply {
        value: json!({"version":1,"generation":null,"freshness":null,"result":null,"error":{"code":code,"message":message.to_string()}}),
        is_error: true,
    })
}
fn tool<T: JsonSchema>(name: &'static str, description: &'static str, read_only: bool) -> Tool {
    let schema = schema_for!(T).to_value();
    let mut tool = Tool::new(
        name,
        description,
        schema.as_object().expect("object schema").clone(),
    );
    tool.annotations = Some(
        ToolAnnotations::new()
            .read_only(read_only)
            .destructive(false)
            .idempotent(read_only)
            .open_world(false),
    );
    tool
}
fn definitions() -> Vec<Tool> {
    vec![
        tool::<CompareArgs>(
            "compare_graphs",
            "Compare a committed base graph against saved WORKTREE (default) or a committed head. Returns full paged differences; carry expected_comparison across pages. Never refreshes or checks out revisions.",
            true,
        ),
        tool::<GraphArgs>(
            "graph_status",
            "Saved graph generation, freshness, counts and diagnostics; never analyzes.",
            true,
        ),
        tool::<SearchArgs>(
            "find_symbols",
            "Bounded stable symbol/class search (limit1..200); query version1.",
            true,
        ),
        tool::<SymbolArgs>(
            "symbol_details",
            "Stable symbol details, locations, certainty/provenance and diagnostics.",
            true,
        ),
        tool::<SectionArgs>(
            "graph_section",
            "Bounded overview, entry-point, neighborhood, path, file or class selection. Explicit filters require version2; limits at most1500nodes/6000edges; depth<=64.",
            true,
        ),
        tool::<NeighborArgs>(
            "neighbors",
            "Bounded incident/adjacent graph results with independent relationship filters.",
            true,
        ),
        tool::<SourceArgs>(
            "symbol_source",
            "CLI-owned analyzed or live source for a stable symbol identity.",
            true,
        ),
        tool::<SymbolArgs>(
            "function_flow",
            "CLI-owned immutable function flow, with unsupported-capability diagnostics.",
            true,
        ),
        tool::<DirectoryArgs>(
            "explore_directory",
            "Paged live explorer through CLI, restricted to configured roots (limit1..1000).",
            true,
        ),
        tool::<Empty>(
            "discover_builds",
            "CLI build discovery including its default_selection identities.",
            true,
        ),
        tool::<RefreshArgs>(
            "refresh_graph",
            "Explicitly analyze discovered build identities and atomically refresh graph. Omit targets to use CLI defaults. Uses fixed startup project/roots.",
            false,
        ),
    ]
}
