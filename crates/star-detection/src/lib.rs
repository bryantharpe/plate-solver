//! Star detection pipeline.
//!
//! This crate implements the front-end image preprocessing for the plate solver:
//! robust per-image noise estimation and a binning cascade for multi-scale
//! detection. Subsequent beads own the `Star` output type, centroiding, and
//! brightness/ordering.

pub mod binning;
pub mod noise;
