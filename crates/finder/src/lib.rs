//! Finder console: a local web page that plate-solves one frame and shows where the
//! telescope points, what is in the field and what to look at next.
//!
//! The `finder` binary serves the page and a small JSON API over one loaded pattern
//! database. The page is a React app in `web/`; its production build (`web/dist/`) is
//! committed and embedded in the binary, so building the crate never needs Node.
//!
//! * `GET /api/health`: the database and how many samples there are.
//! * `GET /api/samples`, `GET /api/samples/{name}`, `GET /api/samples/{name}/thumb`: the
//!   sample frames, and previews of them.
//! * `POST /api/solve`: a multipart frame (an `image` file or a `sample` name) and
//!   options, answered with a [`solve::SolveReport`].
//! * `GET /api/similar?id=M42&lat=&lon=&time=`: objects like a given one, ranked for an
//!   observer, answered with a [`catalog::SimilarReport`].

pub mod api;
pub mod catalog;
pub mod constellation;
pub mod ephemeris;
pub mod grid;
pub mod samples;
pub mod sky;
pub mod solve;
pub mod stars;
pub mod targets;

pub use api::{router, AppState};
