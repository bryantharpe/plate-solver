use clap::Parser;
use pattern_database::load_from_path;
use ps_web::{app, AppState};
use std::path::Path;
use std::sync::Arc;

#[derive(Parser)]
#[command(about = "Plate solver web test harness")]
struct Args {
    /// Path to a tetra3-format pattern database (.npz)
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

    let db = load_from_path(Path::new(&args.db))?;
    eprintln!(
        "loaded database: catalog={} stars={} patterns={} fov=[{}, {}] deg",
        db.properties.star_catalog,
        db.num_stars,
        db.properties.num_patterns,
        db.properties.min_fov,
        db.properties.max_fov
    );

    let state = AppState::new(Arc::new(db));

    eprintln!("plate solver web harness listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app(state)).await?;

    Ok(())
}
