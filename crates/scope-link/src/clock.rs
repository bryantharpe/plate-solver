//! Heartbeat clock mapping using RFC 5905's four timestamps.
//!
//! Times are seconds on their respective clocks. The offset is node time less
//! console time, so mapping a node timestamp subtracts the offset. Callers
//! supply timestamps; this module has no clock or heartbeat scheduler.

/// One heartbeat exchange between the console and scope node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exchange {
    /// Console send time.
    pub t0: f64,
    /// Node receive time.
    pub t1: f64,
    /// Node reply time.
    pub t2: f64,
    /// Console receive time.
    pub t3: f64,
}

impl Exchange {
    /// Round-trip network delay, excluding time spent at the node, in seconds.
    pub fn delay(&self) -> f64 {
        (self.t3 - self.t0) - (self.t2 - self.t1)
    }

    /// Node time less console time, in seconds.
    pub fn offset(&self) -> f64 {
        ((self.t1 - self.t0) + (self.t2 - self.t3)) / 2.0
    }
}

/// Chooses the least-delay exchange in the last eight seconds of supplied t3.
///
/// Push each answered heartbeat (normally every 250 ms). Exchanges exactly
/// eight seconds before the newest t3 remain eligible. Until the first push,
/// the mapping is unknown. All timestamps must be finite seconds.
#[derive(Clone, Debug, Default)]
pub struct ClockMap {
    exchanges: Vec<Exchange>,
}

impl ClockMap {
    /// An empty mapping, with no known offset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an exchange and drops those more than eight seconds old.
    pub fn push(&mut self, exchange: Exchange) {
        self.exchanges.push(exchange);
        let newest_t3 = self
            .exchanges
            .iter()
            .map(|e| e.t3)
            .fold(f64::NEG_INFINITY, f64::max);
        self.exchanges.retain(|e| newest_t3 - e.t3 <= 8.0);
    }

    /// Offset from the eligible exchange with the least network delay.
    pub fn offset(&self) -> Option<f64> {
        self.exchanges
            .iter()
            .min_by(|a, b| a.delay().total_cmp(&b.delay()))
            .map(Exchange::offset)
    }

    /// Maps a node timestamp to the console's clock, once an offset is known.
    pub fn to_console(&self, node_s: f64) -> Option<f64> {
        self.offset().map(|offset| node_s - offset)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockMap, Exchange};

    // US1's worked exchanges are the oracle; every time tolerance is 1 microsecond.
    const TOLERANCE_S: f64 = 1e-6;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < TOLERANCE_S,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn worked_exchanges_choose_the_least_delay_offset() {
        let a = Exchange {
            t0: 100.000,
            t1: 100.070,
            t2: 100.071,
            t3: 100.041,
        };
        let b = Exchange {
            t0: 101.000,
            t1: 101.150,
            t2: 101.151,
            t3: 101.121,
        };
        close(a.delay(), 0.040);
        close(b.delay(), 0.120);
        close(a.offset(), 0.050);
        let mut map = ClockMap::new();
        assert_eq!(map.offset(), None);
        assert_eq!(map.to_console(200.100), None);
        map.push(a);
        map.push(b);
        close(map.offset().unwrap(), 0.050);
    }

    #[test]
    fn nine_second_old_best_exchange_expires() {
        let mut map = ClockMap::default();
        // Hand-worked: 11 ms round trip less 1 ms at the node; offset +20 ms.
        let old = Exchange {
            t0: 190.989,
            t1: 191.014,
            t2: 191.015,
            t3: 191.000,
        };
        close(old.delay(), 0.010);
        close(old.offset(), 0.020);
        map.push(old);
        // The last eight seconds contain only 40 ms delay, +50 ms offset exchanges.
        for quarter in 0..=32 {
            let t3 = 192.0 + f64::from(quarter) * 0.250;
            map.push(Exchange {
                t0: t3 - 0.041,
                t1: t3 + 0.029,
                t2: t3 + 0.030,
                t3,
            });
        }
        close(map.offset().unwrap(), 0.050);
        close(map.to_console(200.100).unwrap(), 200.050);
    }

    #[test]
    fn window_includes_exactly_eight_seconds_and_uses_the_newest_t3() {
        let old = Exchange {
            t0: 190.989,
            t1: 191.014,
            t2: 191.015,
            t3: 191.0,
        };
        let fresh = Exchange {
            t0: 198.959,
            t1: 199.029,
            t2: 199.030,
            t3: 199.0,
        };
        let mut map = ClockMap::new();
        map.push(old);
        map.push(fresh);
        close(map.offset().unwrap(), 0.020);
        map.push(Exchange {
            t0: 199.209,
            t1: 199.279,
            t2: 199.280,
            t3: 199.250,
        });
        close(map.offset().unwrap(), 0.050);
        // An out-of-order reply cannot bring an expired exchange back.
        map.push(old);
        close(map.offset().unwrap(), 0.050);
    }
}
