use clap::Parser;
use scope_node::{
    capture::CaptureLoop,
    clock::SystemClock,
    config::{Adapter, Config},
    rpicam::RpicamCamera,
    synthetic::SyntheticCamera,
    Camera, Settings,
};
use std::{path::PathBuf, process::ExitCode, sync::Arc};

#[derive(Parser)]
#[command(about = "Capture and detect stars on the scope node")]
struct Args {
    #[arg(long, default_value = "/etc/scope-node/config.toml")]
    config: PathBuf,
    /// Print the validated config with the token masked, then exit.
    #[arg(long)]
    check_config: bool,
}

#[tokio::main(worker_threads = 2)]
async fn main() -> ExitCode {
    match start(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("scope-node: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn start(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(&args.config)?;
    if args.check_config {
        print!("{}", config.masked());
        return Ok(());
    }
    let settings = config.settings();
    match config.adapter {
        Adapter::Rpicam => capture(RpicamCamera::discover().await?, settings).await,
        Adapter::Simulated => capture(SyntheticCamera::new(Arc::new(SystemClock)), settings).await,
    }
}

async fn capture(
    camera: impl Camera,
    settings: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut capture = CaptureLoop::new(camera, settings)?;
    loop {
        // CaptureLoop reports adapter errors and retries after one second.
        let _ = capture.next().await;
    }
}
