use clap::ValueEnum;
use r2_core::profile as core_profile;

/// 速度曲線型態（CLI 專用），對應 core 的 VProfile
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VProfile {
    Const,
    Step,
    Sin,
}

pub type ProfileParams = core_profile::ProfileParams;

/// 計算當前時間的期望速度
/// - `profile`: 曲線種類（None 代表 Const）
/// - `params`: 曲線參數
/// - `desired_v_const`: 對於 Const/Step，目標值（Step 在 step_at 之後跳到這個值）
/// - `t`: 目前時間（秒）
pub fn desired_v(profile: Option<VProfile>, params: ProfileParams, desired_v_const: f32, t: f32) -> f32 {
    let mapped = match profile {
        None => None,
        Some(VProfile::Const) => Some(core_profile::VProfile::Const),
        Some(VProfile::Step) => Some(core_profile::VProfile::Step),
        Some(VProfile::Sin) => Some(core_profile::VProfile::Sin),
    };
    core_profile::desired_v(mapped, params, desired_v_const, t)
}

#[derive(Debug)]
pub struct Telemetry {
    pub t: f32,         // 時間戳 (秒)
    pub dt: f32,        // 這次迴圈時間 (秒)
    pub desired_v: f32, // 目標速度
    pub left: f32,      // 左輪輸出（已套用自適應後）
    pub right: f32,     // 右輪輸出（已套用自適應後）
    pub distance: f32,  // 感測距離
    pub state: String,  // FailSafe 狀態

    pub meas_left: f32,  // bench 模式產生的左輪「量測速度」
    pub meas_right: f32, // bench 模式產生的右輪「量測速度」

    pub err: f32,        // 當下（平均）速度誤差：desired_v - v_meas_avg
    pub adapt_gain: f32, // 當下自適應輸出增益（沒開 adaptive 時為 1.0）
}
