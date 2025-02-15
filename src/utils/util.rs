//! # Utility Module
//!
//! This module contains miscellaneous helper functions used throughout RustEd.
//!
//! ## Example Function: `clamp`
//!
//! The `clamp` function restricts a value to lie within a specified range. If the value
//! is below the minimum, it returns the minimum; if it's above the maximum, it returns the
//! maximum; otherwise, it returns the value unchanged.

/// Clamps a value between a minimum and maximum.
///
/// # Arguments
///
/// * `value` - The input value to be clamped.
/// * `min` - The minimum allowable value.
/// * `max` - The maximum allowable value.
///
/// # Examples
///
/// ```
/// use rusted::utils::util::clamp;
///
/// assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
/// assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
/// assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
/// ```
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_within_bounds() {
        // Value is within the range.
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    }

    #[test]
    fn test_clamp_below_bounds() {
        // Value is below the minimum bound.
        assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
    }

    #[test]
    fn test_clamp_above_bounds() {
        // Value is above the maximum bound.
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }
}
