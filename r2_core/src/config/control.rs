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
}
