pub mod plate_solver {
    tonic::include_proto!("plate_solver");
}

pub mod service;
pub use service::PlateSolverService;
