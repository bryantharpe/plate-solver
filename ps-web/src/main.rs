use clap::Parser;
use ps_db::{importer, loader};
use ps_web::{app, AppState};
use std::path::Path;
use std::sync::Arc;

#[derive(Parser)]
#[command(about = "PlateSolver web API")]
struct Args {
    /// Path to the plate solver database (.npz for tetra3 format, otherwise native)
    #[arg(long)]
    db: String,
    /// Address to listen on
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr: std::net::SocketAddr = args.listen.parse()?;

    let db_path = Path::new(&args.db);
    let mut db = if db_path.extension().and_then(|e| e.to_str()) == Some("npz") {
        importer::import_npz(db_path)?
    } else {
        loader::load_native(db_path)?
    };
    db.build_kd_tree();

    let state = AppState::new(Arc::new(db));

    eprintln!("PlateSolver web API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app(state)).await?;

    Ok(())
}
