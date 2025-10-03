use bevy::prelude::*;
use clap::Parser;
use r2_core::config::control::{
    AdaptiveConfig, ControlConfig, FailSafeConfig, PidConfig, SafetyMarginConfig,
};
use r2_core::profile as core_profile;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "sim2d", author, version, about)]
pub struct Cli {
    /// 控制迴圈頻率（Hz）
    #[arg(long, default_value_t = 100.0)]
    pub hz: f32,

    /// 目標線速（m/s）
    #[arg(short = 'v', long, default_value_t = 0.6)]
    pub desired_v: f32,

    /// PID
    #[arg(long, default_value_t = 0.6)]
    pub kp: f32,
    #[arg(long, default_value_t = 0.05)]
    pub ki: f32,
    #[arg(long, default_value_t = 0.0)]
    pub kd: f32,

    /// 速度曲線
    #[arg(long, value_parser=["const","step","sin"], default_value="const", ignore_case = true)]
    pub v_profile: String,
    #[arg(long, default_value_t = 1.0)]
    pub step_at: f32,
    /// Sin 曲線參數（與平台對齊）
    #[arg(long, default_value_t = 0.3)]
    pub sin_amp: f32,
    #[arg(long, default_value_t = 0.2)]
    pub sin_freq: f32,
    #[arg(long, default_value_t = 0.4)]
    pub sin_bias: f32,

    /// FailSafe
    #[arg(long, default_value_t = 0.25)]
    pub threshold: f32,
    #[arg(long, default_value_t = 0.05)]
    pub hysteresis: f32,

    /// 感測安全緩衝比例（相對於車長），例如 0.1 即扣除 10% 車長
    #[arg(long = "safety-margin-ratio", default_value_t = 0.1)]
    pub safety_margin_ratio: f32,

    /// Adaptive
    #[arg(long, default_value_t = false)]
    pub adaptive: bool,
    #[arg(long, default_value_t = 0.02)]
    pub e_small: f32,
    #[arg(long, default_value_t = 0.20)]
    pub e_large: f32,
    #[arg(long, default_value_t = 0.6)]
    pub gain_min: f32,
    #[arg(long, default_value_t = 1.2)]
    pub gain_max: f32,

    /// Bevy world 單位換算（像素/公尺）
    #[arg(long, default_value_t = 100.0)]
    pub px_per_m: f32,

    /// 植物（一階）
    #[arg(long, default_value_t = 0.8)]
    pub tau: f32,
    #[arg(long = "plant-gain", default_value_t = 0.8)]
    pub plant_gain: f32,

    /// CSV 輸出
    #[arg(long, default_value = "run/sim.csv")]
    pub csv: PathBuf,

    /// 多障礙座標（可重複）：格式 "x,y"（世界座標，像素）
    /// 例：--obstacle 300,0 --obstacle 350,120
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

impl Cli {
    pub fn to_runtime(&self) -> RuntimeCfg {
        fn parse_xy(s: &str) -> Option<Vec2> {
            let parts: Vec<_> = s.split(',').collect();
            if parts.len() != 2 {
                return None;
            }
            let x: f32 = parts[0].trim().parse().ok()?;
            let y: f32 = parts[1].trim().parse().ok()?;
            Some(Vec2::new(x, y))
        }
        let obstacles = self.obstacles.iter().filter_map(|s| parse_xy(s)).collect();

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
            obstacles,
        }
    }
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
