//! Telescope and service controls shared by all clones of a simulator.
use crate::{optics, Site, VirtualClock};
use pattern_database::PatternDatabase;
use scope_node::service::NodeControls;
use std::{ops::Deref, sync::Arc};

/// The catalog sky together with the running node's capture and response controls.
#[derive(Clone)]
pub struct Simulator {
    sky: optics::Simulator,
    pub(crate) controls: NodeControls,
}
impl Simulator {
    pub fn new(db: Arc<PatternDatabase>, site: Site, clock: VirtualClock) -> Self {
        Self {
            sky: optics::Simulator::new(db, site, clock),
            controls: NodeControls::default(),
        }
    }
    pub fn push_by(&self, turn: f64, tilt: f64) {
        let current = self.telescope().camera;
        self.push_to((current.alt + tilt).clamp(-90.0, 90.0), current.az + turn);
    }
    pub fn stop_frames(&self) {
        self.controls.stop_frames();
    }
    pub fn start_frames(&self) {
        self.controls.start_frames();
    }
    pub fn set_unresponsive(&self, value: bool) {
        self.controls.set_unresponsive(value);
    }
}
impl Deref for Simulator {
    type Target = optics::Simulator;
    fn deref(&self) -> &Self::Target {
        &self.sky
    }
}
