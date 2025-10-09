use crate::profile::{ProfileExecutor, ProfileParams, VProfile, VelocityProfile};

#[derive(Debug, Clone)]
pub struct LoopConfig {
    pub hz: f32,
    pub desired_v: f32,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            hz: 100.0,
            desired_v: 0.6,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileConfig {
    pub profile: Option<VProfile>,
    pub params: ProfileParams,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            profile: Some(VProfile::Const),
            params: ProfileParams::default(),
        }
    }
}

impl ProfileConfig {
    pub fn executor(&self) -> ProfileExecutor {
        ProfileExecutor::new(self.profile)
    }

    pub fn sample(&self, desired_v_const: f32, t: f32) -> f32 {
        self.executor().sample(self.params, desired_v_const, t)
    }
}

#[derive(Debug, Clone)]
pub struct PlantConfig {
    pub tau: f32,
    pub gain: f32,
}

impl Default for PlantConfig {
    fn default() -> Self {
        Self {
            tau: 0.8,
            gain: 0.8,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoopOverrides {
    pub hz: Option<f32>,
    pub desired_v: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct ProfileOverrides {
    pub profile: Option<Option<VProfile>>,
    pub step_at: Option<f32>,
    pub sin_amp: Option<f32>,
    pub sin_freq: Option<f32>,
    pub sin_bias: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct PlantOverrides {
    pub tau: Option<f32>,
    pub gain: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeOverrides {
    pub loop_cfg: LoopOverrides,
    pub profile: ProfileOverrides,
    pub plant: PlantOverrides,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub loop_cfg: LoopConfig,
    pub profile: ProfileConfig,
    pub plant: PlantConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            loop_cfg: LoopConfig::default(),
            profile: ProfileConfig::default(),
            plant: PlantConfig::default(),
        }
    }
}

impl RuntimeConfig {
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder {
            config: Self::default(),
        }
    }

    pub fn with_overrides(mut self, overrides: &RuntimeOverrides) -> Self {
        self.apply_overrides(overrides);
        self
    }

    pub fn apply_overrides(&mut self, overrides: &RuntimeOverrides) {
        if let Some(hz) = overrides.loop_cfg.hz {
            self.loop_cfg.hz = hz;
        }
        if let Some(desired_v) = overrides.loop_cfg.desired_v {
            self.loop_cfg.desired_v = desired_v;
        }

        if let Some(profile) = overrides.profile.profile {
            self.profile.profile = profile;
        }
        let params = &mut self.profile.params;
        if let Some(step_at) = overrides.profile.step_at {
            params.step_at = step_at;
        }
        if let Some(sin_amp) = overrides.profile.sin_amp {
            params.sin_amp = sin_amp;
        }
        if let Some(sin_freq) = overrides.profile.sin_freq {
            params.sin_freq = sin_freq;
        }
        if let Some(sin_bias) = overrides.profile.sin_bias {
            params.sin_bias = sin_bias;
        }

        if let Some(tau) = overrides.plant.tau {
            self.plant.tau = tau;
        }
        if let Some(gain) = overrides.plant.gain {
            self.plant.gain = gain;
        }
    }
}

pub struct RuntimeBuilder {
    config: RuntimeConfig,
}

impl RuntimeBuilder {
    pub fn new(config: RuntimeConfig) -> Self {
        Self { config }
    }

    pub fn apply(mut self, overrides: &RuntimeOverrides) -> Self {
        self.config.apply_overrides(overrides);
        self
    }

    pub fn build(self) -> RuntimeConfig {
        self.config
    }
}
