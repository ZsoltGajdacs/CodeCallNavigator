//! Independently installable MCP adapter. The installed CLI is its only graph backend.
mod arguments;
mod backend;
mod configuration;
mod process;
mod tools;
mod wire;

use anyhow::Result;
use rmcp::ServiceExt;
#[tokio::main]
async fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => {
            println!(
                "Usage: codecallnavigator-mcp --engine CLI --project ROOT [--root NAME=PATH]...\nStdio MCP server; graph queries use only the configured installed CLI."
            );
            return Ok(());
        }
        [flag] if flag == "--version" => {
            println!("codecallnavigator-mcp {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }
    let config = configuration::Configuration::parse(arguments.into_iter())?;
    let backend = backend::Backend::connect(config).await?;
    let server = tools::Server {
        backend: backend.clone(),
    };
    let result = async {
        let service = server.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        anyhow::Ok(())
    }
    .await;
    backend.shutdown().await;
    result
}
