use clap::ValueEnum;
use r2_core::profile as core_profile;

/// 速度曲線型態（CLI 專用），對應 core 的 VProfile
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VProfile {
    Const,
    Step,
    Sin,
}

#[allow(dead_code)]
pub type ProfileParams = core_profile::ProfileParams;

impl From<VProfile> for core_profile::VProfile {
    fn from(value: VProfile) -> Self {
        match value {
            VProfile::Const => core_profile::VProfile::Const,
            VProfile::Step => core_profile::VProfile::Step,
            VProfile::Sin => core_profile::VProfile::Sin,
        }
    }
}

/// 計算當前時間的期望速度
/// - `profile`: 曲線種類（None 代表 Const）
/// - `params`: 曲線參數
/// - `desired_v_const`: 對於 Const/Step，目標值（Step 在 step_at 之後跳到這個值）
/// - `t`: 目前時間（秒）
#[allow(dead_code)]
pub fn desired_v(
    profile: Option<VProfile>,
    params: ProfileParams,
    desired_v_const: f32,
    t: f32,
) -> f32 {
    let mapped = match profile {
        None => None,
        Some(VProfile::Const) => Some(core_profile::VProfile::Const),
        Some(VProfile::Step) => Some(core_profile::VProfile::Step),
        Some(VProfile::Sin) => Some(core_profile::VProfile::Sin),
    };
    core_profile::desired_v(mapped, params, desired_v_const, t)
}
