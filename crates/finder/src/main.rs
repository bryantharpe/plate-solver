//! `finder`: serve the finder console.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use finder::samples::Samples;
use finder::{router, AppState};

/// Serve the finder console: a page that plate-solves a frame and shows where it points.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Pattern database (.npz) covering your camera's field of view.
    #[arg(
        long,
        default_value = "reference-solutions/cedar-solve/tetra3/data/default_database.npz"
    )]
    db: PathBuf,
    /// Folder of sample frames to offer.
    #[arg(
        long,
        default_value = "reference-solutions/cedar-solve/examples/data/medium_fov"
    )]
    samples: PathBuf,
    /// Address to listen on. 0.0.0.0:8080 lets a phone on the same network connect.
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
    /// Print the third-party notices for the embedded page and data, and exit.
    #[arg(long)]
    licenses: bool,
}

/// The licenses of the third-party works compiled into this binary.
const THIRD_PARTY_NOTICES: &str = include_str!("../THIRD_PARTY_NOTICES.md");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.licenses {
        print!("{THIRD_PARTY_NOTICES}");
        return Ok(());
    }
    let t = Instant::now();
    let db = pattern_database::load_from_path(&args.db)
        .map_err(|e| format!("cannot load the database {}: {e}", args.db.display()))?;
    eprintln!(
        "database: {} stars, {} patterns, FOV {}-{} deg, loaded in {:.1} s",
        db.num_stars,
        db.properties.num_patterns,
        db.properties.min_fov,
        db.properties.max_fov,
        t.elapsed().as_secs_f64()
    );
    let samples = Samples::scan(&args.samples).unwrap_or_else(|e| {
        eprintln!("no samples ({}: {e})", args.samples.display());
        Samples::default()
    });
    eprintln!("samples: {}", samples.len());

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    eprintln!("finder console: http://{}", listener.local_addr()?);
    if args.listen.ip().is_loopback() {
        eprintln!("(to open it from a phone, run with --listen 0.0.0.0:8080)");
    }
    axum::serve(listener, router(AppState::new(db, samples))).await?;
    Ok(())
}
