/// 速度曲線型態（與 CLI 無關，避免在 core 依賴 clap）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        Some(VProfile::Sin) => {
            params.sin_bias + params.sin_amp * (std::f32::consts::TAU * params.sin_freq * t).sin()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desired_const() {
        assert!(
            (desired_v(Some(VProfile::Const), ProfileParams::default(), 0.6, 1.23) - 0.6).abs()
                < 1e-6
        );
        assert!((desired_v(None, ProfileParams::default(), 0.6, 1.23) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn desired_step_edges() {
        let p = ProfileParams {
            step_at: 1.0,
            ..Default::default()
        };
        assert!((desired_v(Some(VProfile::Step), p, 0.6, 0.99) - 0.0).abs() < 1e-6);
        assert!((desired_v(Some(VProfile::Step), p, 0.6, 1.00) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn desired_sin() {
        let p = ProfileParams {
            sin_amp: 0.3,
            sin_freq: 0.2,
            sin_bias: 0.4,
            ..Default::default()
        };
        let v = desired_v(Some(VProfile::Sin), p, 0.0, 0.5);
        assert!(v.is_finite());
    }
}
