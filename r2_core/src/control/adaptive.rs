/// 將 |e| ∈ [e_small, e_large] 線性映射到 [g_min, g_max]，區間外夾限。
#[inline]
pub fn map_gain(abs_e: f32, e_small: f32, e_large: f32, g_min: f32, g_max: f32) -> f32 {
    if abs_e <= e_small {
        g_min
    } else if abs_e >= e_large {
        g_max
    } else {
        let denom = (e_large - e_small).max(1e-12);
        let r = (abs_e - e_small) / denom;
        g_min + r * (g_max - g_min)
    }
}

/// 可插拔的增益排程介面。
pub trait GainScheduler: Send {
    fn update(&mut self, error: f32) -> f32;
    fn last(&self) -> f32;
}

pub type DynGainScheduler = Box<dyn GainScheduler>;

#[derive(Debug, Clone, Copy)]
pub struct AdaptiveParams {
    pub e_small: f32,
    pub e_large: f32,
    pub gain_min: f32,
    pub gain_max: f32,
}

pub struct AdaptiveGainScheduler {
    params: AdaptiveParams,
    last: f32,
}

impl AdaptiveGainScheduler {
    pub fn new(params: AdaptiveParams) -> Self {
        Self { params, last: 1.0 }
    }
}

impl GainScheduler for AdaptiveGainScheduler {
    fn update(&mut self, error: f32) -> f32 {
        let gain = map_gain(
            error.abs(),
            self.params.e_small,
            self.params.e_large,
            self.params.gain_min,
            self.params.gain_max,
        );
        self.last = gain;
        gain
    }

    fn last(&self) -> f32 {
        self.last
    }
}

pub struct FixedGainScheduler {
    gain: f32,
}

impl FixedGainScheduler {
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

impl GainScheduler for FixedGainScheduler {
    fn update(&mut self, _error: f32) -> f32 {
        self.gain
    }

    fn last(&self) -> f32 {
        self.gain
    }
}

pub fn scheduler_from_params(params: Option<AdaptiveParams>) -> DynGainScheduler {
    match params {
        Some(p) => Box::new(AdaptiveGainScheduler::new(p)) as DynGainScheduler,
        None => Box::new(FixedGainScheduler::new(1.0)) as DynGainScheduler,
    }
}

#[cfg(test)]
mod tests {
    use super::{AdaptiveParams, map_gain, scheduler_from_params};

    #[test]
    fn clamps_below_small() {
        assert!((map_gain(0.0, 0.02, 0.2, 0.6, 1.2) - 0.6).abs() < 1e-6);
        assert!((map_gain(0.02, 0.02, 0.2, 0.6, 1.2) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn clamps_above_large() {
        assert!((map_gain(0.3, 0.02, 0.2, 0.6, 1.2) - 1.2).abs() < 1e-6);
        assert!((map_gain(0.2, 0.02, 0.2, 0.6, 1.2) - 1.2).abs() < 1e-6);
    }

    #[test]
    fn interpolates_linearly() {
        let g = map_gain(0.11, 0.02, 0.2, 0.6, 1.2); // 正中間
        assert!((g - 0.9).abs() < 1e-6);
    }

    #[test]
    fn adaptive_scheduler_tracks_error() {
        let params = AdaptiveParams {
            e_small: 0.02,
            e_large: 0.2,
            gain_min: 0.6,
            gain_max: 1.2,
        };
        let mut sched = scheduler_from_params(Some(params));
        let g1 = sched.update(0.0);
        let g2 = sched.update(0.2);
        assert!((g1 - 0.6).abs() < 1e-6);
        assert!((g2 - 1.2).abs() < 1e-6);
        assert!((sched.last() - g2).abs() < 1e-6);
    }

    #[test]
    fn fixed_scheduler_returns_identity() {
        let mut sched = scheduler_from_params(None);
        let g = sched.update(10.0);
        assert!((g - 1.0).abs() < 1e-6);
        assert!((sched.last() - 1.0).abs() < 1e-6);
    }
}
