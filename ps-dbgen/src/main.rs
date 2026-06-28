use clap::Parser;

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

fn main() {
    let args = Args::parse();
    println!("ps-dbgen: catalog={} save={} max_fov={}", args.star_catalog, args.save_as, args.max_fov);
}
