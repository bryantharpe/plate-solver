//! Binomial false-alarm test for candidate attitude acceptance.
//!
//! Implements the statistical gate that decides whether a matched pattern is
//! real or a coincidence, using a binomial CDF evaluated in the deep tail.

/// Result of the false-alarm test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FalseAlarmResult {
    /// Whether the candidate attitude is accepted.
    pub accepted: bool,
    /// The Bonferroni-corrected probability (prob_mismatch * num_patterns).
    pub prob: f64,
    /// The effective acceptance threshold (match_threshold / num_patterns).
    pub effective_threshold: f64,
}

/// Compute the natural logarithm of `n!` for non-negative integers.
///
/// Uses the exact recurrence `ln(n!) = ln((n-1)!) + ln(n)` with a small cache.
/// This avoids the stability and accuracy problems of a generic Lanczos
/// gamma approximation for the small integer arguments that arise in the
/// binomial tail sums.
fn ln_factorial(n: usize) -> f64 {
    // A modest cache covers the sizes typical for plate-solver false-alarm
    // tests (n is the number of extracted centroids). The cache is recomputed
    // on first use and reused across calls in the same thread.
    use std::cell::RefCell;
    thread_local! {
        static CACHE: RefCell<Vec<f64>> = RefCell::new(vec![0.0]); // ln(0!) = 0
    }

    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if n < cache.len() {
            return cache[n];
        }
        let mut value = *cache.last().unwrap();
        for i in cache.len()..=n {
            value += (i as f64).ln();
            cache.push(value);
        }
        cache[n]
    })
}

/// Compute the binomial CDF `P(X <= k)` for `X ~ Binomial(n, p)`.
///
/// Evaluates the smaller of the lower and upper tails in log-space, then
/// exponentiates with a scaled sum. This keeps full precision deep in the
/// tail where naive summation would underflow or cancel catastrophically.
pub fn binomial_cdf(k: usize, n: usize, p: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    if k >= n {
        return 1.0;
    }
    if k == 0 {
        let q = 1.0 - p.clamp(0.0, 1.0);
        return q.powi(n as i32);
    }

    let p = p.clamp(0.0, 1.0);

    // Choose the smaller tail to sum. For p near 1 and k near n, summing the
    // upper tail of the complementary probability is stable.
    if k < n / 2 {
        binomial_lower_tail(k, n, p)
    } else {
        let upper = binomial_lower_tail(n - k - 1, n, 1.0 - p);
        1.0 - upper
    }
}

/// Sum `P(X = 0) + ... + P(X = k)` for `X ~ Binomial(n, p)`.
///
/// Computes in log-space and returns the scaled exponent of the largest term.
fn binomial_lower_tail(k: usize, n: usize, p: f64) -> f64 {
    if k >= n {
        return 1.0;
    }
    if p <= 0.0 {
        return 1.0;
    }
    if p >= 1.0 {
        return if k >= n { 1.0 } else { 0.0 };
    }

    let log_p = p.ln();
    let log_q = (1.0 - p).ln();
    let ln_n_fact = ln_factorial(n);

    let mut max_log = f64::NEG_INFINITY;
    let mut terms = Vec::with_capacity(k + 1);

    for i in 0..=k {
        let ln_binom = ln_n_fact - ln_factorial(i) - ln_factorial(n - i);
        let ln_prob = ln_binom + (i as f64) * log_p + ((n - i) as f64) * log_q;
        terms.push(ln_prob);
        if ln_prob > max_log {
            max_log = ln_prob;
        }
    }

    let sum = terms.iter().map(|&ln_prob| (ln_prob - max_log).exp()).sum::<f64>();
    sum * max_log.exp()
}

/// Run the binomial false-alarm test for a candidate attitude.
///
/// Inputs:
/// * `n` — number of extracted centroids.
/// * `nc` — number of projected nearby catalog stars.
/// * `m` — number of matched stars.
/// * `match_radius` — matching radius as a fraction of image width.
/// * `match_threshold` — base acceptance threshold.
/// * `num_patterns` — number of patterns tried (for Bonferroni correction).
///
/// Computes:
/// * `prob_single = Nc * match_radius^2`
/// * `prob_mismatch = binom.cdf(n - (m - 2), n, 1 - prob_single)`
/// * effective threshold = `match_threshold / num_patterns`
/// * reported `Prob` = `prob_mismatch * num_patterns`
///
/// Returns `accepted = prob_mismatch < effective_threshold`.
pub fn false_alarm_test(
    n: usize,
    nc: usize,
    m: usize,
    match_radius: f64,
    match_threshold: f64,
    num_patterns: usize,
) -> FalseAlarmResult {
    let prob_single = nc as f64 * match_radius * match_radius;
    let prob_single = prob_single.clamp(0.0, 1.0);

    // Degrees of freedom consumed fitting the attitude.
    let dof = 2usize;
    let k = n.saturating_sub(m.saturating_sub(dof));

    let prob_mismatch = binomial_cdf(k, n, 1.0 - prob_single);

    let effective_threshold = if num_patterns == 0 {
        match_threshold
    } else {
        match_threshold / num_patterns as f64
    };
    let prob = prob_mismatch * num_patterns.max(1) as f64;

    FalseAlarmResult {
        accepted: prob_mismatch < effective_threshold,
        prob,
        effective_threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comb(n: usize, k: usize) -> f64 {
        if k > n {
            return 0.0;
        }
        let k = k.min(n - k);
        let mut num = 1.0;
        let mut den = 1.0;
        for i in 0..k {
            num *= (n - i) as f64;
            den *= (i + 1) as f64;
        }
        num / den
    }

    #[test]
    fn binomial_cdf_extremes() {
        assert!((binomial_cdf(0, 10, 0.0) - 1.0).abs() < 1e-12);
        assert!((binomial_cdf(10, 10, 1.0) - 1.0).abs() < 1e-12);
        assert!(binomial_cdf(0, 10, 1.0).abs() < 1e-12);
    }

    #[test]
    fn binomial_cdf_matches_naive_middle() {
        // In the middle of the distribution naive summation is fine.
        let n = 20;
        let p: f64 = 0.3;
        for k in 0..=n {
            let naive: f64 = (0..=k)
                .map(|i| {
                    let c = comb(n, i);
                    c * p.powi(i as i32) * (1.0 - p).powi((n - i) as i32)
                })
                .sum();
            let computed = binomial_cdf(k, n, p);
            assert!(
                (computed - naive).abs() < 1e-9,
                "k={}: computed={}, naive={}",
                k, computed, naive
            );
        }
    }

    #[test]
    fn more_matches_lowers_prob_mismatch() {
        let n = 100;
        let nc = 500;
        let match_radius = 0.02;
        let match_threshold = 1e-3;
        let num_patterns = 1000;

        let mut prev = f64::INFINITY;
        for m in 4..=20 {
            let result = false_alarm_test(n, nc, m, match_radius, match_threshold, num_patterns);
            assert!(
                result.prob < prev || (result.prob - prev).abs() < 1e-15,
                "prob should decrease with m: m={} prob={} prev={}",
                m,
                result.prob,
                prev
            );
            prev = result.prob;
        }
    }

    #[test]
    fn bonferroni_correction() {
        let n = 100;
        let nc = 500;
        let m = 8;
        let match_radius = 0.02;
        let match_threshold = 1e-3;
        let num_patterns = 1000;

        let result = false_alarm_test(n, nc, m, match_radius, match_threshold, num_patterns);

        let prob_single = nc as f64 * match_radius * match_radius;
        let k = n - (m - 2);
        let prob_mismatch = binomial_cdf(k, n, 1.0 - prob_single);

        assert!((result.prob - prob_mismatch * num_patterns as f64).abs() < 1e-12);
        assert!((result.effective_threshold - match_threshold / num_patterns as f64).abs() < 1e-12);
    }

    #[test]
    fn deep_tail_precision() {
        // Parameters chosen to land deep in the tail where naive summation fails.
        let n = 200;
        let nc = 1000;
        let m = 25;
        let match_radius = 0.005;
        let match_threshold = 1e-4;
        let num_patterns = 5000;

        let result = false_alarm_test(n, nc, m, match_radius, match_threshold, num_patterns);

        // The probability should be small but non-zero and finite.
        assert!(result.prob.is_finite());
        assert!(result.prob > 0.0);
        assert!(result.prob < 1.0);

        // With enough matches, should be accepted.
        assert!(result.accepted, "expected accepted: prob={}", result.prob);
    }
}
