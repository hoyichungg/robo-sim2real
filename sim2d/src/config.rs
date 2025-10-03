use bevy::prelude::*;
use clap::{Args, Parser};
use r2_core::config::control::{
    AdaptiveConfig, ControlConfig, FailSafeConfig, PidConfig, SafetyMarginConfig,
};
use r2_core::profile as core_profile;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug, Clone)]
#[command(name = "sim2d", author, version, about)]
pub struct Cli {
    /// Optional TOML config file that provides defaults for the simulation run
    #[arg(
        long,
        value_name = "FILE",
        help = "Load settings from a TOML config file"
    )]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub overrides: OverrideArgs,
}

#[derive(Args, Debug, Clone, Default)]
pub struct OverrideArgs {
    /// Control loop frequency (Hz)
    #[arg(
        long,
        value_name = "HZ",
        help = "Control loop frequency in Hz (default: 100.0)"
    )]
    pub hz: Option<f32>,

    /// Target linear speed (m/s)
    #[arg(
        short = 'v',
        long,
        value_name = "SPEED",
        help = "Target linear velocity in m/s (default: 0.6)"
    )]
    pub desired_v: Option<f32>,

    /// PID gains
    #[arg(long, value_name = "KP", help = "PID proportional gain (default: 0.6)")]
    pub kp: Option<f32>,
    #[arg(long, value_name = "KI", help = "PID integral gain (default: 0.05)")]
    pub ki: Option<f32>,
    #[arg(long, value_name = "KD", help = "PID derivative gain (default: 0.0)")]
    pub kd: Option<f32>,

    /// Velocity profile selection
    #[arg(long, value_parser = ["const", "step", "sin"], ignore_case = true, value_name = "PROFILE", help = "Velocity profile: const | step | sin (default: const)")]
    pub v_profile: Option<String>,
    #[arg(
        long,
        value_name = "SECONDS",
        help = "Step profile switch time in seconds (default: 1.0)"
    )]
    pub step_at: Option<f32>,

    /// Sinusoidal profile parameters (aligned with platform runtime)
    #[arg(
        long,
        value_name = "AMP",
        help = "Sine profile amplitude (default: 0.3)"
    )]
    pub sin_amp: Option<f32>,
    #[arg(
        long,
        value_name = "FREQ",
        help = "Sine profile frequency (Hz) (default: 0.2)"
    )]
    pub sin_freq: Option<f32>,
    #[arg(long, value_name = "BIAS", help = "Sine profile bias (default: 0.4)")]
    pub sin_bias: Option<f32>,

    /// FailSafe parameters
    #[arg(
        long,
        value_name = "METERS",
        help = "Fail-safe stop threshold (default: 0.25)"
    )]
    pub threshold: Option<f32>,
    #[arg(
        long,
        value_name = "METERS",
        help = "Fail-safe hysteresis (default: 0.05)"
    )]
    pub hysteresis: Option<f32>,

    /// Safety margin on sensed distance (ratio of car length)
    #[arg(
        long = "safety-margin-ratio",
        value_name = "RATIO",
        help = "Distance safety margin ratio (default: 0.1)"
    )]
    pub safety_margin_ratio: Option<f32>,

    /// Adaptive controller settings
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::value_parser!(bool),
        value_name = "BOOL",
        help = "Enable or disable adaptive gains (default: false)"
    )]
    pub adaptive: Option<bool>,
    #[arg(
        long,
        value_name = "E_SMALL",
        help = "Adaptive controller small error threshold (default: 0.02)"
    )]
    pub e_small: Option<f32>,
    #[arg(
        long,
        value_name = "E_LARGE",
        help = "Adaptive controller large error threshold (default: 0.20)"
    )]
    pub e_large: Option<f32>,
    #[arg(
        long,
        value_name = "GAIN",
        help = "Adaptive gain minimum (default: 0.6)"
    )]
    pub gain_min: Option<f32>,
    #[arg(
        long,
        value_name = "GAIN",
        help = "Adaptive gain maximum (default: 1.2)"
    )]
    pub gain_max: Option<f32>,

    /// Simulation scaling factors
    #[arg(
        long,
        value_name = "PX_PER_M",
        help = "Pixels per meter in the Bevy world (default: 100.0)"
    )]
    pub px_per_m: Option<f32>,

    /// First-order plant approximation
    #[arg(
        long,
        value_name = "TAU",
        help = "First-order plant time constant (default: 0.8)"
    )]
    pub tau: Option<f32>,
    #[arg(
        long = "plant-gain",
        value_name = "GAIN",
        help = "First-order plant gain (default: 0.8)"
    )]
    pub plant_gain: Option<f32>,

    /// CSV log output path
    #[arg(
        long,
        value_name = "CSV",
        help = "Telemetry CSV output path (default: run/sim.csv)"
    )]
    pub csv: Option<PathBuf>,

    /// Multiple obstacle coordinates, e.g. --obstacle 300,0 --obstacle 350,120
    #[arg(long = "obstacle", value_name = "X,Y")]
    pub obstacles: Vec<String>,
}

#[derive(Resource, Debug, Clone)]
pub struct RuntimeCfg {
    pub hz: f32,
    pub desired_v: f32,
    pub control: ControlConfig,
    pub v_profile: String,
    pub step_at: f32,
    pub sin_amp: f32,
    pub sin_freq: f32,
    pub sin_bias: f32,
    pub tau: f32,
    pub plant_gain: f32,
    pub px_per_m: f32,
    pub csv: String,
    pub obstacles: Vec<Vec2>,
}

#[derive(Debug, Clone)]
pub struct SimSettings {
    pub hz: f32,
    pub desired_v: f32,
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub v_profile: String,
    pub step_at: f32,
    pub sin_amp: f32,
    pub sin_freq: f32,
    pub sin_bias: f32,
    pub threshold: f32,
    pub hysteresis: f32,
    pub safety_margin_ratio: f32,
    pub adaptive: bool,
    pub e_small: f32,
    pub e_large: f32,
    pub gain_min: f32,
    pub gain_max: f32,
    pub px_per_m: f32,
    pub tau: f32,
    pub plant_gain: f32,
    pub csv: PathBuf,
    pub obstacles: Vec<Vec2>,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            hz: 100.0,
            desired_v: 0.6,
            kp: 0.6,
            ki: 0.05,
            kd: 0.0,
            v_profile: "const".to_string(),
            step_at: 1.0,
            sin_amp: 0.3,
            sin_freq: 0.2,
            sin_bias: 0.4,
            threshold: 0.25,
            hysteresis: 0.05,
            safety_margin_ratio: 0.1,
            adaptive: false,
            e_small: 0.02,
            e_large: 0.20,
            gain_min: 0.6,
            gain_max: 1.2,
            px_per_m: 100.0,
            tau: 0.8,
            plant_gain: 0.8,
            csv: PathBuf::from("run/sim.csv"),
            obstacles: Vec::new(),
        }
    }
}

impl SimSettings {
    pub fn to_runtime(&self) -> RuntimeCfg {
        let mut pid_cfg = PidConfig::default();
        pid_cfg.kp = self.kp;
        pid_cfg.ki = self.ki;
        pid_cfg.kd = self.kd;

        let mut failsafe_cfg = FailSafeConfig::default();
        failsafe_cfg.threshold_m = self.threshold;
        failsafe_cfg.hysteresis_m = self.hysteresis;

        let mut adaptive_cfg = AdaptiveConfig::default();
        adaptive_cfg.enabled = self.adaptive;
        adaptive_cfg.e_small = self.e_small;
        adaptive_cfg.e_large = self.e_large;
        adaptive_cfg.gain_min = self.gain_min;
        adaptive_cfg.gain_max = self.gain_max;

        let mut safety_margin = SafetyMarginConfig::default();
        safety_margin.ratio_of_car_length = self.safety_margin_ratio;

        let control = ControlConfig {
            pid: pid_cfg,
            failsafe: failsafe_cfg,
            adaptive: adaptive_cfg,
            safety_margin,
        };

        RuntimeCfg {
            hz: self.hz,
            desired_v: self.desired_v,
            control,
            v_profile: self.v_profile.clone(),
            step_at: self.step_at,
            sin_amp: self.sin_amp,
            sin_freq: self.sin_freq,
            sin_bias: self.sin_bias,
            tau: self.tau,
            plant_gain: self.plant_gain,
            px_per_m: self.px_per_m,
            csv: self.csv.to_string_lossy().to_string(),
            obstacles: self.obstacles.clone(),
        }
    }

    pub fn csv_path(&self) -> &Path {
        &self.csv
    }

    fn apply_file(&mut self, cfg: FileOverrides, base_dir: &Path) -> Result<(), String> {
        if let Some(value) = cfg.hz {
            self.hz = value;
        }
        if let Some(value) = cfg.desired_v {
            self.desired_v = value;
        }
        if let Some(value) = cfg.kp {
            self.kp = value;
        }
        if let Some(value) = cfg.ki {
            self.ki = value;
        }
        if let Some(value) = cfg.kd {
            self.kd = value;
        }
        if let Some(value) = cfg.v_profile {
            self.v_profile = value;
        }
        if let Some(value) = cfg.step_at {
            self.step_at = value;
        }
        if let Some(value) = cfg.sin_amp {
            self.sin_amp = value;
        }
        if let Some(value) = cfg.sin_freq {
            self.sin_freq = value;
        }
        if let Some(value) = cfg.sin_bias {
            self.sin_bias = value;
        }
        if let Some(value) = cfg.threshold {
            self.threshold = value;
        }
        if let Some(value) = cfg.hysteresis {
            self.hysteresis = value;
        }
        if let Some(value) = cfg.safety_margin_ratio {
            self.safety_margin_ratio = value;
        }
        if let Some(value) = cfg.adaptive {
            self.adaptive = value;
        }
        if let Some(value) = cfg.e_small {
            self.e_small = value;
        }
        if let Some(value) = cfg.e_large {
            self.e_large = value;
        }
        if let Some(value) = cfg.gain_min {
            self.gain_min = value;
        }
        if let Some(value) = cfg.gain_max {
            self.gain_max = value;
        }
        if let Some(value) = cfg.px_per_m {
            self.px_per_m = value;
        }
        if let Some(value) = cfg.tau {
            self.tau = value;
        }
        if let Some(value) = cfg.plant_gain {
            self.plant_gain = value;
        }
        if let Some(value) = cfg.csv {
            let resolved = if value.is_relative() {
                base_dir.join(value)
            } else {
                value
            };
            self.csv = resolved;
        }
        if let Some(list) = cfg.obstacles {
            let mut obstacles = Vec::with_capacity(list.len());
            for item in list {
                let vec2 = item.to_vec2()?;
                obstacles.push(vec2);
            }
            self.obstacles = obstacles;
        }
        Ok(())
    }

    fn apply_cli(&mut self, overrides: &OverrideArgs) -> Result<(), String> {
        if let Some(value) = overrides.hz {
            self.hz = value;
        }
        if let Some(value) = overrides.desired_v {
            self.desired_v = value;
        }
        if let Some(value) = overrides.kp {
            self.kp = value;
        }
        if let Some(value) = overrides.ki {
            self.ki = value;
        }
        if let Some(value) = overrides.kd {
            self.kd = value;
        }
        if let Some(value) = overrides.v_profile.clone() {
            self.v_profile = value;
        }
        if let Some(value) = overrides.step_at {
            self.step_at = value;
        }
        if let Some(value) = overrides.sin_amp {
            self.sin_amp = value;
        }
        if let Some(value) = overrides.sin_freq {
            self.sin_freq = value;
        }
        if let Some(value) = overrides.sin_bias {
            self.sin_bias = value;
        }
        if let Some(value) = overrides.threshold {
            self.threshold = value;
        }
        if let Some(value) = overrides.hysteresis {
            self.hysteresis = value;
        }
        if let Some(value) = overrides.safety_margin_ratio {
            self.safety_margin_ratio = value;
        }
        if let Some(value) = overrides.adaptive {
            self.adaptive = value;
        }
        if let Some(value) = overrides.e_small {
            self.e_small = value;
        }
        if let Some(value) = overrides.e_large {
            self.e_large = value;
        }
        if let Some(value) = overrides.gain_min {
            self.gain_min = value;
        }
        if let Some(value) = overrides.gain_max {
            self.gain_max = value;
        }
        if let Some(value) = overrides.px_per_m {
            self.px_per_m = value;
        }
        if let Some(value) = overrides.tau {
            self.tau = value;
        }
        if let Some(value) = overrides.plant_gain {
            self.plant_gain = value;
        }
        if let Some(value) = overrides.csv.clone() {
            self.csv = value;
        }
        if !overrides.obstacles.is_empty() {
            let obstacles = parse_obstacle_strings(&overrides.obstacles)?;
            self.obstacles = obstacles;
        }
        Ok(())
    }
}

impl Cli {
    pub fn into_settings(self) -> Result<SimSettings, String> {
        let mut settings = SimSettings::default();

        if let Some(path) = self.config.as_ref() {
            let cfg = load_file_config(path)?;
            let base = path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            settings.apply_file(cfg, &base)?;
        }

        settings.apply_cli(&self.overrides)?;
        Ok(settings)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct FileOverrides {
    hz: Option<f32>,
    desired_v: Option<f32>,
    kp: Option<f32>,
    ki: Option<f32>,
    kd: Option<f32>,
    v_profile: Option<String>,
    step_at: Option<f32>,
    sin_amp: Option<f32>,
    sin_freq: Option<f32>,
    sin_bias: Option<f32>,
    threshold: Option<f32>,
    hysteresis: Option<f32>,
    safety_margin_ratio: Option<f32>,
    adaptive: Option<bool>,
    e_small: Option<f32>,
    e_large: Option<f32>,
    gain_min: Option<f32>,
    gain_max: Option<f32>,
    px_per_m: Option<f32>,
    tau: Option<f32>,
    plant_gain: Option<f32>,
    csv: Option<PathBuf>,
    obstacles: Option<Vec<ObstacleSpec>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ObstacleSpec {
    Pair([f32; 2]),
    Map { x: f32, y: f32 },
    Text(String),
}

impl ObstacleSpec {
    fn to_vec2(&self) -> Result<Vec2, String> {
        match self {
            ObstacleSpec::Pair([x, y]) => Ok(Vec2::new(*x, *y)),
            ObstacleSpec::Map { x, y } => Ok(Vec2::new(*x, *y)),
            ObstacleSpec::Text(s) => parse_obstacle_string(s),
        }
    }
}

fn load_file_config(path: &Path) -> Result<FileOverrides, String> {
    let data = fs::read_to_string(path)
        .map_err(|err| format!("failed to read config file {}: {}", path.display(), err))?;
    toml::from_str(&data)
        .map_err(|err| format!("failed to parse config file {}: {}", path.display(), err))
}

fn parse_obstacle_strings(values: &[String]) -> Result<Vec<Vec2>, String> {
    values.iter().map(|s| parse_obstacle_string(s)).collect()
}

fn parse_obstacle_string(value: &str) -> Result<Vec2, String> {
    let parts: Vec<_> = value.split(',').collect();
    if parts.len() != 2 {
        return Err(format!(
            "invalid obstacle '{}', expected format 'x,y'",
            value
        ));
    }
    let x: f32 = parts[0]
        .trim()
        .parse()
        .map_err(|err| format!("invalid x value in obstacle '{}': {}", value, err))?;
    let y: f32 = parts[1]
        .trim()
        .parse()
        .map_err(|err| format!("invalid y value in obstacle '{}': {}", value, err))?;
    Ok(Vec2::new(x, y))
}

pub fn desired_speed(cfg: &RuntimeCfg, t: f32) -> f32 {
    let prof = match cfg.v_profile.to_ascii_lowercase().as_str() {
        "const" => Some(core_profile::VProfile::Const),
        "step" => Some(core_profile::VProfile::Step),
        "sin" => Some(core_profile::VProfile::Sin),
        _ => None,
    };
    let params = core_profile::ProfileParams {
        step_at: cfg.step_at,
        sin_amp: cfg.sin_amp,
        sin_freq: cfg.sin_freq,
        sin_bias: cfg.sin_bias,
    };
    core_profile::desired_v(prof, params, cfg.desired_v, t)
}
