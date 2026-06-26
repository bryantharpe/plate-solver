//! Binomial false-alarm test (doc 02 §8).
//!
//! Estimates the probability that a set of star matches occurred by chance,
//! using a binomial CDF model. Includes Bonferroni-corrected helpers for
//! multi-pattern comparison.

/// Probability that a single star is a mismatch:
/// prob_single = num_nearby_catalog_stars * match_radius^2
///
/// Binomial CDF: P(X <= k) where X ~ Binomial(n, 1 - prob_single)
/// with k = n - (m - 2), n = num_extracted_stars, m = num_star_matches.
///
/// The -2 accounts for two degrees of freedom consumed by fitting attitude.
///
/// Return the probability of mismatch (lower = more confident match).
///
/// IMPORTANT: Clamp prob_single to [0, 1]. If k < 0, return 0.0 (auto-accept).
pub fn false_alarm_probability(
    num_extracted_stars: usize,      // n — total image centroids
    num_nearby_catalog_stars: usize, // Nc — catalog stars projected in-frame
    num_star_matches: usize,         // m — number of 1-to-1 matches
    match_radius: f64,               // fraction of image width
) -> f64 {
    let n = num_extracted_stars;
    if n == 0 {
        return 1.0;
    }

    let k = if num_star_matches >= 2 {
        n as isize - (num_star_matches as isize - 2)
    } else {
        // m < 2 means k > n, which is a trivially high mismatch probability
        n as isize + (2isize - num_star_matches as isize)
    };

    if k < 0 {
        return 0.0;
    }
    let k_usize = k as usize;

    // prob_single = Nc * match_radius^2, clamped to [0, 1]
    let prob_single = (num_nearby_catalog_stars as f64) * match_radius * match_radius;
    let prob_single = prob_single.clamp(0.0, 1.0);

    // p = 1 - prob_single (probability a star is NOT a mismatch)
    let p = 1.0 - prob_single;

    // Binomial CDF: P(X <= k) for X ~ Binomial(n, p)
    binom_cdf(k_usize, n, p)
}

/// Binomial CDF: P(X <= k) for X ~ Binomial(n, p).
///
/// Uses iterative computation for numerical stability:
/// P(0) = (1-p)^n, then P(i+1) = P(i) * (n-i)/(i+1) * p/(1-p)
fn binom_cdf(k: usize, n: usize, p: f64) -> f64 {
    if k >= n {
        return 1.0;
    }
    if p == 0.0 {
        return 1.0; // P(X=0) = 1, so P(X <= k) = 1 for any k >= 0
    }
    if p == 1.0 {
        return if k >= n { 1.0 } else { 0.0 };
    }

    let q = 1.0 - p; // probability of "failure"

    // Start with P(0) = C(n, 0) * p^0 * q^n = q^n
    let mut prob = q.powi(n as i32);
    let mut cdf = prob;

    for i in 0..k {
        // P(i+1) = P(i) * (n - i) / (i + 1) * p / q
        prob = prob * (n - i) as f64 / (i as f64 + 1.0) * p / q;
        cdf += prob;
    }

    cdf
}

/// Effective match threshold with Bonferroni correction:
/// threshold / num_patterns
pub fn effective_match_threshold(user_threshold: f64, num_patterns: usize) -> f64 {
    user_threshold / num_patterns as f64
}

/// Reported probability (Bonferroni-corrected):
/// prob_mismatch * num_patterns
pub fn reported_probability(prob_mismatch: f64, num_patterns: usize) -> f64 {
    prob_mismatch * num_patterns as f64
}
