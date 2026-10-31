//! The node and SensorTimestamp share CLOCK_MONOTONIC's boot-relative epoch.

pub trait NodeClock: Send + Sync {
    fn now(&self) -> f64;
}

#[derive(Clone, Copy, Default)]
pub struct SystemClock;

impl NodeClock for SystemClock {
    fn now(&self) -> f64 {
        let time = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
        time.tv_sec as f64 + time.tv_nsec as f64 / 1e9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_uses_the_sensor_epoch_without_a_process_local_origin() {
        let seconds = || {
            let t = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
            t.tv_sec as f64 + t.tv_nsec as f64 / 1e9
        };
        let before = seconds();
        let now = SystemClock.now();
        let after = seconds();
        assert!((before..=after).contains(&now));
    }
}
