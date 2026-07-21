//! `tetra3-gen-db` command-line entry point.
//!
//! Mirrors the reference `tetra3-gen-db` CLI: parses a star catalog, runs the
//! offline pattern-database generation pipeline, and writes the resulting `.npz`
//! file. All generation parameters are exposed as flags so the same catalog and
//! parameters reproduce a byte-identical database.

use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

use database_generation::{
    build_pattern_catalog, clean_and_limit, derive_magnitude_limit, enumerate_patterns, parse_bsc5,
    parse_hip, parse_tyc, propagate, serialize_to_path, CatalogEntry, CatalogSource,
    GenerationConfig, SerializeError,
};
use pattern_database::DatabaseProperties;

/// Generate a star-pattern database from a star catalog.
#[derive(Debug, Parser)]
#[command(
    name = "tetra3-gen-db",
    about = "Generate a tetra3 pattern database from a star catalog"
)]
struct Cli {
    /// Path to the star catalog file (BSC5 binary, Hipparcos pipe-delimited,
    /// or Tycho-2 pipe-delimited).
    star_catalog: PathBuf,

    /// Path where the generated `.npz` database will be written.
    save_as: PathBuf,

    /// Maximum horizontal FOV (degrees) the database must support.
    #[arg(
        long,
        required = true,
        help = "Maximum angle between stars in the same pattern"
    )]
    max_fov: f64,

    /// Minimum horizontal FOV (degrees) considered when trimming catalogue density.
    /// Defaults to `--max-fov` if omitted.
    #[arg(long, help = "Minimum FOV considered when trimming catalogue density")]
    min_fov: Option<f64>,

    /// Use a linear-probe hash table sized `next_prime(3·N)` instead of the
    /// quadratic default `next_prime(2·N)`.
    #[arg(long, help = "Use a linear-probe hash table")]
    linear_probe: bool,

    /// Target epoch for proper-motion propagation. Use `none` to disable
    /// propagation, or a year like `2026.5` to pin the epoch for reproducible
    /// builds. Defaults to the current year.
    #[arg(long, value_parser = parse_epoch, help = "Epoch for proper-motion propagation (none|<year>)")]
    epoch_proper_motion: Option<EpochArg>,

    /// Dimmest apparent magnitude to retain. When omitted, the limit is derived
    /// from `min_fov` and `verification_stars_per_fov`.
    #[arg(long, help = "Dimmest apparent magnitude retained")]
    star_max_magnitude: Option<f64>,

    /// Target number of verification stars per FOV-sized region.
    #[arg(
        long,
        default_value_t = 150.0,
        help = "Target verification stars per FOV"
    )]
    verification_stars_per_fov: f64,

    /// Lattice-field oversampling factor.
    #[arg(
        long,
        default_value_t = 100.0,
        help = "Lattice field oversampling factor"
    )]
    lattice_field_oversampling: f64,

    /// Number of patterns to generate per lattice field.
    #[arg(
        long,
        default_value_t = 50,
        help = "Patterns generated per lattice field"
    )]
    patterns_per_lattice_field: usize,

    /// Maximum allowed pattern error; determines `pattern_bins`.
    #[arg(long, default_value_t = 0.001, help = "Maximum allowed pattern error")]
    pattern_max_error: f64,

    /// Largest allowed ratio between subsequent FOVs in a multiscale database.
    #[arg(long, default_value_t = 1.5, help = "Multiscale FOV step factor")]
    multiscale_step: f64,
}

#[derive(Debug, Clone, Copy)]
enum EpochArg {
    None,
    Year(f64),
}

fn parse_epoch(s: &str) -> Result<EpochArg, String> {
    if s.eq_ignore_ascii_case("none") {
        Ok(EpochArg::None)
    } else {
        s.parse::<f64>()
            .map(EpochArg::Year)
            .map_err(|e| format!("invalid epoch: {e}"))
    }
}

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Serialize(SerializeError),
    Usage(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Serialize(e) => write!(f, "{e}"),
            Error::Usage(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<SerializeError> for Error {
    fn from(e: SerializeError) -> Self {
        Error::Serialize(e)
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let source = detect_catalog_source(&cli.star_catalog)?;
    let catalog_name = catalog_name(&cli.star_catalog, source);

    let mut entries = read_catalog(&cli.star_catalog, source)?;
    if entries.is_empty() {
        return Err(Error::Usage(
            "star catalog contains no usable entries".to_string(),
        ));
    }

    let min_fov = cli.min_fov.unwrap_or(cli.max_fov);
    if min_fov > cli.max_fov {
        return Err(Error::Usage(
            "--min-fov cannot be larger than --max-fov".to_string(),
        ));
    }
    if min_fov <= 0.0 || cli.max_fov <= 0.0 {
        return Err(Error::Usage("FOV values must be positive".to_string()));
    }

    // Proper-motion propagation.
    let epoch_proper_motion = match cli.epoch_proper_motion {
        Some(EpochArg::None) => None,
        Some(EpochArg::Year(y)) => Some(y),
        None => Some(database_generation::config::current_year()),
    };
    let pm_origin = match source {
        CatalogSource::Bsc5 => 2000.0,
        CatalogSource::Hip | CatalogSource::Tyc => 1991.25,
    };
    if let Some(epoch) = epoch_proper_motion {
        propagate(&mut entries, pm_origin, epoch);
    }

    // Cleanup, sort, and magnitude limiting.
    let mut config = GenerationConfig {
        epoch_proper_motion,
        star_max_magnitude: cli.star_max_magnitude,
        min_fov,
        verification_stars_per_fov: cli.verification_stars_per_fov,
    };
    clean_and_limit(&mut entries, config.star_max_magnitude);
    if config.star_max_magnitude.is_none() {
        let total_stars_needed = database_generation::num_fields_for_sky(config.min_fov)
            * config.verification_stars_per_fov
            * 0.7;
        config.star_max_magnitude = derive_magnitude_limit(&entries, total_stars_needed);
    }
    if let Some(limit) = config.star_max_magnitude {
        clean_and_limit(&mut entries, Some(limit));
    }
    if entries.is_empty() {
        return Err(Error::Usage(
            "no stars remain after magnitude limiting".to_string(),
        ));
    }

    // Pattern enumeration and catalog construction.
    let patterns = enumerate_patterns(
        &entries,
        min_fov,
        cli.max_fov,
        cli.verification_stars_per_fov,
        cli.lattice_field_oversampling,
        cli.patterns_per_lattice_field,
    );
    if patterns.is_empty() {
        return Err(Error::Usage(
            "no patterns generated from the catalog".to_string(),
        ));
    }

    let catalog =
        build_pattern_catalog(&entries, &patterns, cli.pattern_max_error, cli.linear_probe);

    let properties = DatabaseProperties {
        pattern_mode: "edge_ratio".to_string(),
        hash_table_type: if cli.linear_probe {
            "linear_probe".to_string()
        } else {
            "quadratic_probe".to_string()
        },
        pattern_size: 4,
        pattern_bins: catalog.pattern_bins as u16,
        pattern_max_error: cli.pattern_max_error as f32,
        max_fov: cli.max_fov as f32,
        min_fov: min_fov as f32,
        star_catalog: catalog_name,
        epoch_equinox: 2000,
        epoch_proper_motion: epoch_proper_motion.unwrap_or(pm_origin) as f32,
        verification_stars_per_fov: cli.verification_stars_per_fov as u16,
        star_max_magnitude: config.star_max_magnitude.unwrap_or(0.0) as f32,
        num_patterns: catalog.num_patterns as u32,
    };

    serialize_to_path(&cli.save_as, &entries, &catalog, &properties)?;
    Ok(())
}

fn detect_catalog_source(path: &Path) -> Result<CatalogSource, Error> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if name.contains("bsc5") || name == "bsc" {
        Ok(CatalogSource::Bsc5)
    } else if name.contains("tyc") || name.starts_with("tyc") {
        Ok(CatalogSource::Tyc)
    } else if name.contains("hip") || name.starts_with("hip") {
        Ok(CatalogSource::Hip)
    } else {
        // Fall back to content sniffing: BSC5 starts with a 28-byte binary header,
        // while HIP/TYC are pipe-delimited text.
        let mut file = BufReader::new(File::open(path)?);
        let mut header = [0u8; 28];
        let n = file.read(&mut header)?;
        if n >= 28 {
            // BSC5: the third 32-bit word (STARN) is non-zero and the header is exactly 28 bytes.
            let starn = i32::from_le_bytes([header[8], header[9], header[10], header[11]]);
            if starn != 0 {
                return Ok(CatalogSource::Bsc5);
            }
        }
        Err(Error::Usage(format!(
            "could not detect catalog type for '{}'; expected bsc5, hip_main, or tyc_main in the file name",
            path.display()
        )))
    }
}

fn catalog_name(path: &Path, source: CatalogSource) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    match source {
        CatalogSource::Bsc5 if !stem.to_ascii_lowercase().contains("bsc5") => "bsc5".to_string(),
        CatalogSource::Hip if !stem.to_ascii_lowercase().contains("hip") => "hip_main".to_string(),
        CatalogSource::Tyc if !stem.to_ascii_lowercase().contains("tyc") => "tyc_main".to_string(),
        _ => stem,
    }
}

fn read_catalog(path: &Path, source: CatalogSource) -> Result<Vec<CatalogEntry>, Error> {
    let file = File::open(path)?;
    let entries = match source {
        CatalogSource::Bsc5 => parse_bsc5(file)?,
        CatalogSource::Hip => parse_hip(file)?,
        CatalogSource::Tyc => parse_tyc(file)?,
    };
    Ok(entries)
}
