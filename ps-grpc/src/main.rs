use clap::Parser;
use ps_db::loader;
use ps_grpc::plate_solver::plate_solver_server::PlateSolverServer;
use ps_grpc::PlateSolverService;
use std::path::Path;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;

#[derive(Parser)]
#[command(about = "PlateSolver gRPC service")]
struct Args {
    /// Bind address (host:port)
    #[arg(long, default_value = "127.0.0.1:50051")]
    address: String,
    /// Path to the native plate solver database
    #[arg(long)]
    db_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr: std::net::SocketAddr = args.address.parse()?;

    let mut db = loader::load_native(Path::new(&args.db_path))?;
    db.build_kd_tree();
    let svc = PlateSolverService::new(db);

    eprintln!("PlateSolver gRPC listening on {}", addr);
    Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(PlateSolverServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
