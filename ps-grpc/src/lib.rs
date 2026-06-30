pub mod plate_solver {
    tonic::include_proto!("plate_solver");
}

pub mod cedar_detect {
    tonic::include_proto!("cedar_detect");
}

pub mod service;
pub use service::PlateSolverService;
