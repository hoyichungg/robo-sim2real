use bevy::prelude::*;
use clap::{Args, Parser};
use r2_core::config::control::{ControlConfig, ControlOverrides};
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

impl OverrideArgs {
    fn control_overrides(&self) -> ControlOverrides {
        let mut overrides = ControlOverrides::default();
        overrides.pid.kp = self.kp;
        overrides.pid.ki = self.ki;
        overrides.pid.kd = self.kd;
        overrides.failsafe.threshold_m = self.threshold;
        overrides.failsafe.hysteresis_m = self.hysteresis;
        overrides.safety_margin.ratio_of_car_length = self.safety_margin_ratio;
        overrides.adaptive.enabled = self.adaptive;
        overrides.adaptive.e_small = self.e_small;
        overrides.adaptive.e_large = self.e_large;
        overrides.adaptive.gain_min = self.gain_min;
        overrides.adaptive.gain_max = self.gain_max;
        overrides
    }
}

#[derive(Resource, Debug, Clone)]
pub struct RuntimeCfg {
    pub hz: f32,
    pub desired_v: f32,
    pub control: ControlConfig,
    pub v_profile: Option<core_profile::VProfile>,
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
    pub control: ControlConfig,
    pub v_profile: Option<core_profile::VProfile>,
    pub step_at: f32,
    pub sin_amp: f32,
    pub sin_freq: f32,
    pub sin_bias: f32,
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
            control: ControlConfig::default(),
            v_profile: Some(core_profile::VProfile::Const),
            step_at: 1.0,
            sin_amp: 0.3,
            sin_freq: 0.2,
            sin_bias: 0.4,
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
        RuntimeCfg {
            hz: self.hz,
            desired_v: self.desired_v,
            control: self.control,
            v_profile: self.v_profile,
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
        let control_overrides = cfg.control_overrides();

        if let Some(value) = cfg.hz {
            self.hz = value;
        }
        if let Some(value) = cfg.desired_v {
            self.desired_v = value;
        }
        if let Some(value) = cfg.v_profile {
            let parsed = parse_v_profile(&value)?;
            self.v_profile = Some(parsed);
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

        self.control.apply_overrides(&control_overrides);
        Ok(())
    }

    fn apply_cli(&mut self, overrides: &OverrideArgs) -> Result<(), String> {
        let control_overrides = overrides.control_overrides();

        if let Some(value) = overrides.hz {
            self.hz = value;
        }
        if let Some(value) = overrides.desired_v {
            self.desired_v = value;
        }
        if let Some(value) = overrides.v_profile.as_deref() {
            let parsed = parse_v_profile(value)?;
            self.v_profile = Some(parsed);
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

        self.control.apply_overrides(&control_overrides);
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

impl FileOverrides {
    fn control_overrides(&self) -> ControlOverrides {
        let mut overrides = ControlOverrides::default();
        overrides.pid.kp = self.kp;
        overrides.pid.ki = self.ki;
        overrides.pid.kd = self.kd;
        overrides.failsafe.threshold_m = self.threshold;
        overrides.failsafe.hysteresis_m = self.hysteresis;
        overrides.safety_margin.ratio_of_car_length = self.safety_margin_ratio;
        overrides.adaptive.enabled = self.adaptive;
        overrides.adaptive.e_small = self.e_small;
        overrides.adaptive.e_large = self.e_large;
        overrides.adaptive.gain_min = self.gain_min;
        overrides.adaptive.gain_max = self.gain_max;
        overrides
    }
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

fn parse_v_profile(raw: &str) -> Result<core_profile::VProfile, String> {
    let trimmed = raw.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "const" => Ok(core_profile::VProfile::Const),
        "step" => Ok(core_profile::VProfile::Step),
        "sin" => Ok(core_profile::VProfile::Sin),
        _ => Err(format!("invalid velocity profile '{}'", trimmed)),
    }
}

pub fn desired_speed(cfg: &RuntimeCfg, t: f32) -> f32 {
    let params = core_profile::ProfileParams {
        step_at: cfg.step_at,
        sin_amp: cfg.sin_amp,
        sin_freq: cfg.sin_freq,
        sin_bias: cfg.sin_bias,
    };
    core_profile::desired_v(cfg.v_profile, params, cfg.desired_v, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const EPS: f32 = 1e-6;

    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < EPS,
            "{label} mismatch: got {actual}, expected {expected}"
        );
    }

    fn assert_control_eq(actual: &ControlConfig, expected: &ControlConfig) {
        assert_close(actual.pid.kp, expected.pid.kp, "pid.kp");
        assert_close(actual.pid.ki, expected.pid.ki, "pid.ki");
        assert_close(actual.pid.kd, expected.pid.kd, "pid.kd");
        assert_close(
            actual.pid.limits.out_min,
            expected.pid.limits.out_min,
            "pid.limits.out_min",
        );
        assert_close(
            actual.pid.limits.out_max,
            expected.pid.limits.out_max,
            "pid.limits.out_max",
        );
        assert_close(
            actual.pid.limits.i_min,
            expected.pid.limits.i_min,
            "pid.limits.i_min",
        );
        assert_close(
            actual.pid.limits.i_max,
            expected.pid.limits.i_max,
            "pid.limits.i_max",
        );

        assert_close(
            actual.failsafe.threshold_m,
            expected.failsafe.threshold_m,
            "failsafe.threshold_m",
        );
        assert_close(
            actual.failsafe.hysteresis_m,
            expected.failsafe.hysteresis_m,
            "failsafe.hysteresis_m",
        );

        assert_eq!(
            actual.adaptive.enabled, expected.adaptive.enabled,
            "adaptive.enabled mismatch"
        );
        assert_close(
            actual.adaptive.e_small,
            expected.adaptive.e_small,
            "adaptive.e_small",
        );
        assert_close(
            actual.adaptive.e_large,
            expected.adaptive.e_large,
            "adaptive.e_large",
        );
        assert_close(
            actual.adaptive.gain_min,
            expected.adaptive.gain_min,
            "adaptive.gain_min",
        );
        assert_close(
            actual.adaptive.gain_max,
            expected.adaptive.gain_max,
            "adaptive.gain_max",
        );

        assert_close(
            actual.safety_margin.ratio_of_car_length,
            expected.safety_margin.ratio_of_car_length,
            "safety_margin.ratio_of_car_length",
        );
    }

    #[test]
    fn file_overrides_merge_into_control_config() {
        let mut settings = SimSettings::default();
        let file = FileOverrides {
            kp: Some(1.25),
            ki: Some(0.15),
            kd: Some(0.07),
            threshold: Some(0.18),
            hysteresis: Some(0.06),
            safety_margin_ratio: Some(0.35),
            adaptive: Some(true),
            e_small: Some(0.01),
            e_large: Some(0.45),
            gain_min: Some(0.55),
            gain_max: Some(1.65),
            ..Default::default()
        };
        let expected_overrides = file.control_overrides();
        settings
            .apply_file(file, Path::new("."))
            .expect("file overrides should apply");
        let expected = ControlConfig::default().with_overrides(&expected_overrides);
        assert_control_eq(&settings.control, &expected);
    }

    #[test]
    fn cli_overrides_merge_into_control_config() {
        let mut settings = SimSettings::default();
        let overrides = OverrideArgs {
            kp: Some(1.4),
            ki: Some(0.22),
            kd: Some(0.11),
            threshold: Some(0.21),
            hysteresis: Some(0.08),
            safety_margin_ratio: Some(0.28),
            adaptive: Some(true),
            e_small: Some(0.03),
            e_large: Some(0.52),
            gain_min: Some(0.7),
            gain_max: Some(1.9),
            ..Default::default()
        };
        let expected_overrides = overrides.control_overrides();
        settings
            .apply_cli(&overrides)
            .expect("cli overrides should apply");
        let expected = ControlConfig::default().with_overrides(&expected_overrides);
        assert_control_eq(&settings.control, &expected);
    }

    #[test]
    fn cli_overrides_take_precedence_over_file() {
        let mut settings = SimSettings::default();
        let file = FileOverrides {
            kp: Some(0.9),
            ki: Some(0.12),
            kd: Some(0.05),
            threshold: Some(0.25),
            hysteresis: Some(0.05),
            safety_margin_ratio: Some(0.3),
            adaptive: Some(false),
            e_small: Some(0.02),
            e_large: Some(0.4),
            gain_min: Some(0.5),
            gain_max: Some(1.4),
            ..Default::default()
        };
        let file_overrides = file.control_overrides();
        settings
            .apply_file(file, Path::new("."))
            .expect("file overrides should apply");

        let overrides = OverrideArgs {
            kp: Some(1.8),
            kd: Some(0.2),
            threshold: Some(0.3),
            adaptive: Some(true),
            gain_max: Some(2.1),
            ..Default::default()
        };
        let cli_overrides = overrides.control_overrides();
        settings
            .apply_cli(&overrides)
            .expect("cli overrides should apply");

        let mut expected = ControlConfig::default();
        expected.apply_overrides(&file_overrides);
        expected.apply_overrides(&cli_overrides);

        assert_control_eq(&settings.control, &expected);
    }
}
