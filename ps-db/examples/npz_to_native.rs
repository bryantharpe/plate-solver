//! Convert a reference tetra3/cedar-solve `.npz` pattern database to ps-db's
//! native binary format.
//!
//! Used by the feat-09 eval-harness to produce a shared catalog the ps-grpc
//! and cedar-flow benchmark adapters both solve against, via the same
//! `import_npz` -> `save_native` path `ps-grpc/src/service.rs` already uses
//! to build its own test database.

use std::path::PathBuf;

use clap::Parser;
use ps_db::{importer, loader};

#[derive(Parser)]
#[command(
    name = "npz_to_native",
    about = "Convert a tetra3/cedar-solve .npz pattern database to ps-db's native format"
)]
struct Args {
    /// Input reference database (.npz)
    npz_path: PathBuf,
    /// Output path for the native database (.bin)
    save_as: PathBuf,
}

fn main() {
    let args = Args::parse();

    let db = importer::import_npz(&args.npz_path)
        .unwrap_or_else(|e| panic!("import_npz({:?}) failed: {}", args.npz_path, e));

    if let Some(parent) = args.save_as.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("create_dir_all({:?}) failed: {}", parent, e));
    }

    loader::save_native(&db, &args.save_as)
        .unwrap_or_else(|e| panic!("save_native({:?}) failed: {}", args.save_as, e));

    println!(
        "Wrote {:?}: {} stars, {} hash slots, {} patterns, hash_table_type={}",
        args.save_as,
        db.num_stars(),
        db.num_slots(),
        db.properties.num_patterns,
        db.properties.hash_table_type,
    );
}
