use anyhow::Result;
use rmcp::{
    RoleClient, serve_client,
    service::RunningService,
    transport::{ConfigureCommandExt, TokioChildProcess},
};

use stakpak_shared::Env;
use tokio::process::Command;

pub async fn local_client(env: Env) -> Result<RunningService<RoleClient, ()>> {
    let process = match env {
        Env::Dev => TokioChildProcess::new(Command::new("cargo").configure(|cmd| {
            cmd.arg("run");
            cmd.arg("mcp");
        }))?,
        Env::Prod => TokioChildProcess::new(Command::new("stakpak").configure(|cmd| {
            cmd.arg("mcp");
        }))?,
    };
    let service = serve_client((), process).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.peer_info();

    Ok(service)
}
