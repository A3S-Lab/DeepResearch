const ONE_SIDED_95_PERCENT_Z: f64 = 1.644_853_626_951_472_2;
const ONE_SIDED_95_PERCENT_T_AT_47_DF: f64 = 1.677_926_721_641_86;

pub(crate) fn rate_bps(numerator: usize, denominator: usize) -> u16 {
    numerator
        .saturating_mul(10_000)
        .checked_div(denominator)
        .unwrap_or_default()
        .min(10_000) as u16
}

pub(crate) fn rate_bps_ceil(numerator: usize, denominator: usize) -> u16 {
    numerator
        .saturating_mul(10_000)
        .saturating_add(denominator.saturating_sub(1))
        .checked_div(denominator)
        .unwrap_or_default()
        .min(10_000) as u16
}

pub(crate) fn wilson_lower_bps(successes: usize, samples: usize) -> u16 {
    if samples == 0 {
        return 0;
    }
    let n = samples as f64;
    let observed = successes as f64 / n;
    let z = ONE_SIDED_95_PERCENT_Z;
    let denominator = 1.0 + z * z / n;
    let centre = observed + z * z / (2.0 * n);
    let margin = z * ((observed * (1.0 - observed) + z * z / (4.0 * n)) / n).sqrt();
    (((centre - margin) / denominator * 10_000.0)
        .floor()
        .max(0.0)) as u16
}

pub(crate) fn paired_delta_lower_milli(deltas: &[f64]) -> i16 {
    if deltas.len() < 2 {
        return i16::MIN;
    }
    let count = deltas.len() as f64;
    let mean = deltas.iter().sum::<f64>() / count;
    let variance = deltas
        .iter()
        .map(|delta| (delta - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    let lower = mean - ONE_SIDED_95_PERCENT_T_AT_47_DF * (variance / count).sqrt();
    lower
        .floor()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

#[cfg(test)]
mod tests {
    use super::{paired_delta_lower_milli, rate_bps_ceil};

    #[test]
    fn maximum_failure_rates_round_up() {
        assert_eq!(rate_bps_ceil(26, 519), 501);
    }

    #[test]
    fn paired_noninferiority_uses_a_conservative_small_sample_bound() {
        let mut deltas = vec![-1_000.0; 9];
        deltas.extend(vec![44.0; 39]);
        assert!(paired_delta_lower_milli(&deltas) < -250);
    }
}
