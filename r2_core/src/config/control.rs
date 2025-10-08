use crate::control::pid::Pid;
use crate::control::safety::FailSafe;

#[derive(Debug, Clone, Copy)]
pub struct PidLimits {
    pub out_min: f32,
    pub out_max: f32,
    pub i_min: f32,
    pub i_max: f32,
}

impl Default for PidLimits {
    fn default() -> Self {
        Self {
            out_min: -1.0,
            out_max: 1.0,
            i_min: -0.5,
            i_max: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PidConfig {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub limits: PidLimits,
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            kp: 0.6,
            ki: 0.05,
            kd: 0.0,
            limits: PidLimits::default(),
        }
    }
}

impl PidConfig {
    pub fn build(self) -> Pid {
        Pid::new(self.kp, self.ki, self.kd)
            .with_output_limits(self.limits.out_min, self.limits.out_max)
            .with_integral_limits(self.limits.i_min, self.limits.i_max)
    }

    pub fn apply_overrides(&mut self, overrides: &PidOverride) {
        if let Some(kp) = overrides.kp {
            self.kp = kp;
        }
        if let Some(ki) = overrides.ki {
            self.ki = ki;
        }
        if let Some(kd) = overrides.kd {
            self.kd = kd;
        }
        if let Some(lims) = overrides.limits {
            if let Some(out_min) = lims.out_min {
                self.limits.out_min = out_min;
            }
            if let Some(out_max) = lims.out_max {
                self.limits.out_max = out_max;
            }
            if let Some(i_min) = lims.i_min {
                self.limits.i_min = i_min;
            }
            if let Some(i_max) = lims.i_max {
                self.limits.i_max = i_max;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FailSafeConfig {
    pub threshold_m: f32,
    pub hysteresis_m: f32,
}

impl Default for FailSafeConfig {
    fn default() -> Self {
        Self {
            threshold_m: 0.25,
            hysteresis_m: 0.05,
        }
    }
}

impl FailSafeConfig {
    pub fn build(self) -> FailSafe {
        FailSafe::new(self.threshold_m, self.hysteresis_m)
    }

    pub fn apply_overrides(&mut self, overrides: &FailSafeOverride) {
        if let Some(threshold) = overrides.threshold_m {
            self.threshold_m = threshold;
        }
        if let Some(hysteresis) = overrides.hysteresis_m {
            self.hysteresis_m = hysteresis;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AdaptiveConfig {
    pub enabled: bool,
    pub e_small: f32,
    pub e_large: f32,
    pub gain_min: f32,
    pub gain_max: f32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            e_small: 0.02,
            e_large: 0.20,
            gain_min: 0.6,
            gain_max: 1.2,
        }
    }
}

impl AdaptiveConfig {
    pub fn apply_overrides(&mut self, overrides: &AdaptiveOverride) {
        if let Some(enabled) = overrides.enabled {
            self.enabled = enabled;
        }
        if let Some(e_small) = overrides.e_small {
            self.e_small = e_small;
        }
        if let Some(e_large) = overrides.e_large {
            self.e_large = e_large;
        }
        if let Some(gain_min) = overrides.gain_min {
            self.gain_min = gain_min;
        }
        if let Some(gain_max) = overrides.gain_max {
            self.gain_max = gain_max;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SafetyMarginConfig {
    pub ratio_of_car_length: f32,
}

impl Default for SafetyMarginConfig {
    fn default() -> Self {
        Self {
            ratio_of_car_length: 0.1,
        }
    }
}

impl SafetyMarginConfig {
    pub fn apply_overrides(&mut self, overrides: &SafetyMarginOverride) {
        if let Some(ratio) = overrides.ratio_of_car_length {
            self.ratio_of_car_length = ratio;
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ControlConfig {
    pub pid: PidConfig,
    pub failsafe: FailSafeConfig,
    pub adaptive: AdaptiveConfig,
    pub safety_margin: SafetyMarginConfig,
}

impl ControlConfig {
    pub fn build_pid(&self) -> Pid {
        self.pid.build()
    }

    pub fn build_failsafe(&self) -> FailSafe {
        self.failsafe.build()
    }

    pub fn margin_ratio(&self) -> f32 {
        self.safety_margin.ratio_of_car_length.max(0.0)
    }

    pub fn adaptive_enabled(&self) -> bool {
        self.adaptive.enabled
    }

    pub fn adaptive_params(&self) -> (f32, f32, f32, f32) {
        (
            self.adaptive.e_small,
            self.adaptive.e_large,
            self.adaptive.gain_min,
            self.adaptive.gain_max,
        )
    }

    pub fn apply_overrides(&mut self, overrides: &ControlOverrides) {
        self.pid.apply_overrides(&overrides.pid);
        self.failsafe.apply_overrides(&overrides.failsafe);
        self.adaptive.apply_overrides(&overrides.adaptive);
        self.safety_margin.apply_overrides(&overrides.safety_margin);
    }

    pub fn with_overrides(mut self, overrides: &ControlOverrides) -> Self {
        self.apply_overrides(overrides);
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PidOverride {
    pub kp: Option<f32>,
    pub ki: Option<f32>,
    pub kd: Option<f32>,
    pub limits: Option<PidLimitsOverride>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PidLimitsOverride {
    pub out_min: Option<f32>,
    pub out_max: Option<f32>,
    pub i_min: Option<f32>,
    pub i_max: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FailSafeOverride {
    pub threshold_m: Option<f32>,
    pub hysteresis_m: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AdaptiveOverride {
    pub enabled: Option<bool>,
    pub e_small: Option<f32>,
    pub e_large: Option<f32>,
    pub gain_min: Option<f32>,
    pub gain_max: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SafetyMarginOverride {
    pub ratio_of_car_length: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ControlOverrides {
    pub pid: PidOverride,
    pub failsafe: FailSafeOverride,
    pub adaptive: AdaptiveOverride,
    pub safety_margin: SafetyMarginOverride,
}
