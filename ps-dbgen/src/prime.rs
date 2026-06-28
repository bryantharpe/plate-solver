//! Prime number utilities for hash table sizing.

/// Check if a number is prime using trial division.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5u64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}

/// Return the smallest prime >= `n`.
pub fn next_prime(n: u64) -> u64 {
    let mut candidate = if n < 2 { 2 } else { n + 1 };
    while !is_prime(candidate) {
        candidate += 1;
    }
    candidate
}
