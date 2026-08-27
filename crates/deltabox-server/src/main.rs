use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use deltabox_core::Vault;
use deltabox_server::{build_router, AppState};

#[derive(Parser)]
#[command(name = "deltabox-server", about = "deltabox H5 web server")]
struct Args {
    /// Vault root directory (must already be initialized)
    #[arg(long, default_value = ".")]
    vault: PathBuf,

    /// Listen address
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Listen port
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Directory with the built frontend (SPA)
    #[arg(long, default_value = "apps/web/dist")]
    static_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let vault = Vault::open(&args.vault).with_context(|| {
        format!(
            "failed to open vault at {} (run `deltabox --vault <path> init` first)",
            args.vault.display()
        )
    })?;

    if !args.static_dir.join("index.html").exists() {
        eprintln!(
            "warning: no frontend found at {} (build it with `cd apps/web && pnpm build`); only /api will work",
            args.static_dir.display()
        );
    }

    let app = build_router(AppState { vault }, args.static_dir);
    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("deltabox server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
