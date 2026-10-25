/// About this many steps between the lowest and highest tick.
const STEPS: f64 = 4.0;
/// Room above and below the data, as a share of its range.
const MARGIN: f64 = 0.1;

/// A value axis: round ticks from `lo` to `hi`, `step` apart.
#[derive(Debug, PartialEq)]
pub struct Scale {
    pub lo: f64,
    pub hi: f64,
    pub step: f64,
}

impl Scale {
    /// Covers `min..=max` with some room around it. Doesn't go below zero
    /// for data that doesn't.
    pub fn new(min: f64, max: f64) -> Self {
        let span = max - min;
        let margin = if span > 0.0 {
            span * MARGIN
        } else if max != 0.0 {
            max.abs() * MARGIN
        } else {
            0.5
        };
        let mut lo = min - margin;
        if min >= 0.0 {
            lo = lo.max(0.0);
        }
        let hi = max + margin;
        let step = round_step((hi - lo) / STEPS);
        Self {
            lo: (lo / step).floor() * step,
            hi: (hi / step).ceil() * step,
            step,
        }
    }

    /// From `lo` up to `hi`.
    pub fn ticks(&self) -> impl Iterator<Item = f64> {
        let count = ((self.hi - self.lo) / self.step).round() as usize;
        (0..=count).map(move |i| self.lo + i as f64 * self.step)
    }

    /// Where `value` is, from 0 at `lo` to 1 at `hi`.
    pub fn position(&self, value: f64) -> f64 {
        (value - self.lo) / (self.hi - self.lo)
    }

    /// Enough to tell the ticks apart: 0 for a step of 5, 2 for 0.05.
    pub fn decimals(&self) -> usize {
        (-(self.step.log10() + 1e-9).floor()).max(0.0) as usize
    }
}

/// 1, 2 or 5 times a power of ten, at least `raw`.
fn round_step(raw: f64) -> f64 {
    let magnitude = 10f64.powf(raw.log10().floor());
    let factor = match raw / magnitude {
        f if f <= 1.0 => 1.0,
        f if f <= 2.0 => 2.0,
        f if f <= 5.0 => 5.0,
        _ => 10.0,
    };
    factor * magnitude
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(min: f64, max: f64) -> Vec<String> {
        let scale = Scale::new(min, max);
        let decimals = scale.decimals();
        scale.ticks().map(|t| format!("{t:.decimals$}")).collect()
    }

    #[test]
    fn small_percentages_get_small_round_steps() {
        assert_eq!(ticks(0.05, 0.17), ["0.00", "0.05", "0.10", "0.15", "0.20"]);
    }

    #[test]
    fn a_narrow_band_far_from_zero_keeps_its_detail() {
        assert_eq!(ticks(300.0, 320.0), ["290", "300", "310", "320", "330"]);
    }

    #[test]
    fn flat_data_still_gets_a_range() {
        assert_eq!(ticks(0.0, 0.0), ["0.0", "0.2", "0.4", "0.6"]);
        assert_eq!(ticks(50.0, 50.0), ["45", "50", "55"]);
    }

    #[test]
    fn negative_data_can_go_below_zero() {
        assert_eq!(ticks(-5.0, 5.0), ["-10", "-5", "0", "5", "10"]);
    }
}
