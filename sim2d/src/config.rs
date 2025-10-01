use bevy::prelude::*;
use clap::Parser;
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
    #[arg(long, default_value_t = 0.04)]
    pub kd: f32,

    /// 速度曲線
    #[arg(long, value_parser=["const","step","sin"], default_value="const")]
    pub v_profile: String,
    #[arg(long, default_value_t = 1.0)]
    pub step_at: f32,

    /// FailSafe
    #[arg(long, default_value_t = 0.25)]
    pub threshold: f32,
    #[arg(long, default_value_t = 0.05)]
    pub hysteresis: f32,

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
    pub px_per_m: f32, // 1 m = 幾個 Bevy world 單位（像素）

    /// 植物（一階）
    #[arg(long, default_value_t = 0.8)]
    pub tau: f32,
    #[arg(long = "plant-gain", default_value_t = 0.8)]
    pub plant_gain: f32,

    /// CSV 輸出
    #[arg(long, default_value = "run/sim.csv")]
    pub csv: PathBuf,
}

#[derive(Resource, Debug, Clone)]
pub struct RuntimeCfg {
    pub hz: f32,
    pub desired_v: f32,
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub v_profile: String,
    pub step_at: f32,
    pub threshold: f32,
    pub hysteresis: f32,
    pub adaptive: bool,
    pub e_small: f32,
    pub e_large: f32,
    pub gain_min: f32,
    pub gain_max: f32,
    pub tau: f32,
    pub plant_gain: f32,
    pub px_per_m: f32,
    pub csv: String,
}

impl Cli {
    pub fn to_runtime(&self) -> RuntimeCfg {
        RuntimeCfg {
            hz: self.hz,
            desired_v: self.desired_v,
            kp: self.kp,
            ki: self.ki,
            kd: self.kd,
            v_profile: self.v_profile.clone(),
            step_at: self.step_at,
            threshold: self.threshold,
            hysteresis: self.hysteresis,
            adaptive: self.adaptive,
            e_small: self.e_small,
            e_large: self.e_large,
            gain_min: self.gain_min,
            gain_max: self.gain_max,
            tau: self.tau,
            plant_gain: self.plant_gain,
            px_per_m: self.px_per_m,
            csv: self.csv.to_string_lossy().to_string(),
        }
    }
}

pub fn desired_speed(cfg: &RuntimeCfg, t: f32) -> f32 {
    match cfg.v_profile.as_str() {
        "step" => {
            if t >= cfg.step_at {
                cfg.desired_v
            } else {
                0.0
            }
        }
        _ => cfg.desired_v,
    }
}
