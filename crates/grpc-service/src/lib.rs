//! gRPC service surface for the plate solver.
//!
//! This crate owns the `PlateSolver` service definition and the boundary between
//! the wire protocol and the solver. It is responsible for:
//!
//! * Protobuf message shapes (`Image`, `ImageCoord`, `StarCentroid`, `Solution`,
//!   `SolveParams`, ...).
//! * The four RPCs: `ExtractCentroids`, `SolveFromCentroids`, `SolveFromImage`,
//!   and `GetInfo`.
//! * The **coordinate boundary swap**: gRPC clients speak `(x, y)` while the
//!   solver speaks `(y, x)`. The swap happens here and nowhere else.
//! * Forwarding detection/solve parameters to the solver.
//!
//! The shared-memory fast path and full `SolveFromImage` detection fidelity are
//! not implemented yet; the service returns explicit errors for them.

pub mod proto {
    tonic::include_proto!("plate_solver");
}

pub mod server;

pub use server::PlateSolverServer;
