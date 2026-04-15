//! STDIO transport for the MCP server.

use anyhow::Result;
use rmcp::ServiceExt;

use crate::server::{{ ProjectName }}Server;

pub async fn serve_stdio(server: {{ ProjectName }}Server) -> Result<()> {
    tracing::info!("starting MCP stdio transport");

    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    tracing::info!("stdio transport closed");
    Ok(())
}
