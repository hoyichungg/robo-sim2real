use clap::ValueEnum;
use std::f32::consts::TAU;

/// 速度曲線型態
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VProfile {
    Const,
    Step,
    Sin,
}

/// 各曲線的參數（用在 Step/Sin）
#[derive(Debug, Clone, Copy)]
pub struct ProfileParams {
    pub step_at: f32,  // 階躍開始時間 (s)
    pub sin_amp: f32,  // 正弦幅度
    pub sin_freq: f32, // 正弦頻率 (Hz)
    pub sin_bias: f32, // 正弦偏置
}

impl Default for ProfileParams {
    fn default() -> Self {
        Self {
            step_at: 1.0,
            sin_amp: 0.3,
            sin_freq: 0.2,
            sin_bias: 0.4,
        }
    }
}

/// 計算當前時間的期望速度
/// - `profile`: 曲線種類（None 代表 Const）
/// - `params`: 曲線參數
/// - `desired_v_const`: 對於 Const/Step，目標值（Step 在 step_at 之後跳到這個值）
/// - `t`: 目前時間（秒）
pub fn desired_v(
    profile: Option<VProfile>,
    params: ProfileParams,
    desired_v_const: f32,
    t: f32,
) -> f32 {
    match profile {
        None | Some(VProfile::Const) => desired_v_const,
        Some(VProfile::Step) => {
            if t >= params.step_at {
                desired_v_const
            } else {
                0.0
            }
        }
        Some(VProfile::Sin) => params.sin_bias + params.sin_amp * (TAU * params.sin_freq * t).sin(),
    }
}
