use std::collections::HashSet;

use clap::Parser;
use ps_core::pattern::compute_pattern_bins;

#[cfg(feature = "kd-tree")]
use ps_dbgen::fov::{compute_fov_ladder, thin_stars_for_fov};
#[cfg(feature = "kd-tree")]
use ps_dbgen::hash_insert::build_hash_table;
#[cfg(feature = "kd-tree")]
use ps_dbgen::patterns::enumerate_patterns;

#[cfg(feature = "kd-tree")]
use ps_db::Database;

use ps_dbgen::catalog::{ParseParams, StarRecord};
use ps_dbgen::cleanup::auto_limiting_magnitude;
use ps_dbgen::vectors::compute_star_vectors;

#[derive(Parser, Debug)]
#[command(name = "ps-dbgen", about = "Build a plate-solver pattern database")]
struct Args {
    /// Input star catalog file (BSC5 binary, HIP .dat, or TYC .dat)
    star_catalog: String,
    /// Output database path
    save_as: String,
    /// Maximum FOV in degrees
    #[arg(long)]
    max_fov: f64,
    /// Minimum FOV in degrees
    #[arg(long)]
    min_fov: Option<f64>,
    /// Use linear probing instead of quadratic
    #[arg(long, default_value_t = false)]
    linear_probe: bool,
}

/// Detect catalog format from file extension.
fn detect_catalog_format(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".bsc5") || lower.contains("bsc5") {
        "bsc5"
    } else if lower.ends_with(".dat") && lower.contains("tyc") {
        "tyc"
    } else if lower.ends_with(".dat") || lower.contains("hip") {
        "hip"
    } else {
        "hip" // default fallback
    }
}

/// Parse a star catalog file into StarRecords.
fn parse_catalog(path: &str, params: &ParseParams) -> Vec<StarRecord> {
    let format = detect_catalog_format(path);
    let file = std::fs::File::open(path).expect("failed to open catalog file");
    match format {
        "bsc5" => {
            let mut reader = std::io::BufReader::new(file);
            ps_dbgen::catalog::bsc5::parse_bsc5(&mut reader, params)
                .expect("failed to parse BSC5 catalog")
        }
        "tyc" => {
            ps_dbgen::catalog::tyc::parse_tyc(file, params).expect("failed to parse TYC catalog")
        }
        _ => ps_dbgen::catalog::hip::parse_hip(file, params).expect("failed to parse HIP catalog"),
    }
}

fn main() {
    let args = Args::parse();

    let min_fov = args.min_fov.unwrap_or(args.max_fov * 0.33);
    println!(
        "ps-dbgen: catalog={} save={} max_fov={} min_fov={}",
        args.star_catalog, args.save_as, args.max_fov, min_fov
    );

    // Step 1: Parse catalog
    let params = ParseParams::default();
    let mut stars = parse_catalog(&args.star_catalog, &params);
    println!("Parsed {} stars", stars.len());

    // Step 2: Sort by magnitude (brightest first) and apply auto limiting magnitude
    stars.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap());

    let min_fov_rad = min_fov * std::f64::consts::PI / 180.0;
    let verification_stars_per_fov = 30usize;
    let limiting_mag = auto_limiting_magnitude(&stars, min_fov_rad, verification_stars_per_fov);
    println!("Auto limiting magnitude: {:.2}", limiting_mag);

    // Filter by limiting magnitude
    stars.retain(|s| s.mag <= limiting_mag);
    println!("Stars after magnitude cut: {}", stars.len());

    // Step 3: Compute unit vectors
    let star_vectors = compute_star_vectors(&stars);
    println!("Computed {} unit vectors", star_vectors.len());

    #[cfg(feature = "kd-tree")]
    {
        // Step 4: Build FOV ladder
        let max_fov_rad = args.max_fov * std::f64::consts::PI / 180.0;
        let min_fov_rad_for_ladder = min_fov * std::f64::consts::PI / 180.0;
        let multiscale_step = 1.5_f64;
        let fov_ladder = compute_fov_ladder(min_fov_rad_for_ladder, max_fov_rad, multiscale_step);
        println!(
            "FOV ladder (degrees): {:?}",
            fov_ladder
                .iter()
                .map(|f| f * 180.0 / std::f64::consts::PI)
                .collect::<Vec<_>>()
        );

        // Step 5: Enumerate patterns per FOV scale
        let lattice_field_oversampling = 100usize;
        let patterns_per_lattice_field = 50usize;
        let stars_per_fov = 40usize;
        let pattern_max_error = 0.001_f64;
        let pattern_bins = compute_pattern_bins(pattern_max_error);

        let mut all_patterns: HashSet<[usize; 4]> = HashSet::new();

        for &fov_rad in &fov_ladder {
            let fov_deg = fov_rad * 180.0 / std::f64::consts::PI;
            println!(
                "Enumerating patterns for FOV={:.2}° (rad={:.4})",
                fov_deg, fov_rad
            );

            // Density thinning for this FOV
            let thin_indices = thin_stars_for_fov(&star_vectors, fov_rad, stars_per_fov);
            println!("  Thinned to {} stars", thin_indices.len());

            // Map thin indices back to a contiguous vector slice
            let pattern_vectors: Vec<[f32; 3]> =
                thin_indices.iter().map(|&i| star_vectors[i]).collect();

            // Enumerate patterns (indices are into pattern_vectors, need to map back to global)
            let fov_patterns = enumerate_patterns(
                &pattern_vectors,
                fov_rad,
                lattice_field_oversampling,
                patterns_per_lattice_field,
            );

            // Map local indices back to global star indices
            for pat in &fov_patterns {
                let global_pat: [usize; 4] = pat.map(|local_idx| thin_indices[local_idx]);
                all_patterns.insert(global_pat);
            }

            println!(
                "  Found {} unique patterns for this FOV",
                fov_patterns.len()
            );
        }

        println!(
            "Total unique patterns across all scales: {}",
            all_patterns.len()
        );

        // Step 6: Build hash table
        let hash_result = build_hash_table(
            &all_patterns,
            &star_vectors,
            pattern_bins,
            args.linear_probe,
        );
        println!(
            "Hash table built: {} slots, {} patterns inserted",
            hash_result.key_hashes.len(),
            hash_result.num_patterns
        );

        // Step 7: Populate Database and save
        let num_stars = stars.len();
        let mut star_table = Vec::with_capacity(num_stars);
        for (i, star) in stars.iter().enumerate() {
            star_table.push([
                star.ra as f32,
                star.dec as f32,
                star_vectors[i][0],
                star_vectors[i][1],
                star_vectors[i][2],
                star.mag as f32,
            ]);
        }

        // Determine catalog name from file
        let catalog_name = detect_catalog_format(&args.star_catalog).to_string();

        let properties = ps_db::DatabaseProperties {
            pattern_mode: "edge_ratio".into(),
            hash_table_type: if args.linear_probe {
                "linear_probe".into()
            } else {
                "quadratic_probe".into()
            },
            pattern_size: 4,
            pattern_bins: pattern_bins as u16,
            pattern_max_error: pattern_max_error as f32,
            max_fov: args.max_fov as f32,
            min_fov: min_fov as f32,
            star_catalog: catalog_name,
            epoch_equinox: 2000,
            epoch_proper_motion: params.epoch_proper_motion as f32,
            lattice_field_oversampling: lattice_field_oversampling as u16,
            patterns_per_lattice_field: patterns_per_lattice_field as u16,
            verification_stars_per_fov: verification_stars_per_fov as u16,
            star_max_magnitude: limiting_mag as f32,
            presort_patterns: true,
            num_patterns: hash_result.num_patterns,
        };

        let mut db = Database::empty(properties);
        db.star_table = star_table;
        db.pattern_catalog_u8 = hash_result.catalog_u8;
        db.pattern_catalog_u16 = hash_result.catalog_u16;
        db.pattern_catalog_u32 = hash_result.catalog_u32;
        db.largest_edge = hash_result.largest_edge;
        db.key_hashes = hash_result.key_hashes;

        // Save native format
        let save_path = std::path::Path::new(&args.save_as);
        ps_db::loader::save_native(&db, save_path).expect("failed to save database");
        println!("Database saved to {}", args.save_as);
    }

    #[cfg(not(feature = "kd-tree"))]
    {
        println!("kd-tree feature not enabled; skipping pattern enumeration and hash table build.");
        let _ = (
            stars,
            star_vectors,
            args.max_fov,
            min_fov,
            args.linear_probe,
        );
    }
}
