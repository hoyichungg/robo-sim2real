#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyState {
    Run,
    EmergencyBrake,
    SafeStop,
}

#[derive(Debug, Clone)]
pub struct FailSafe {
    pub threshold_m: f32,  // 觸發門檻
    pub hysteresis_m: f32, // 解除回滯：需 > threshold + hysteresis 才允許解除
    state: SafetyState,
}

impl FailSafe {
    pub fn new(threshold_m: f32, hysteresis_m: f32) -> Self {
        Self {
            threshold_m,
            hysteresis_m,
            state: SafetyState::Run,
        }
    }

    pub fn state(&self) -> SafetyState {
        self.state
    }

    /// 將任何錯誤/NaN/負值視為危險。
    pub fn update(&mut self, distance: Result<f32, ()>) -> SafetyState {
        // 接受 +∞ 表示「非常遠」；NaN 或錯誤視為危險；負值視為 0
        let d_ok = match distance {
            Ok(d) if d.is_nan() => None,
            Ok(d) => Some(d.max(0.0)),
            Err(_) => None,
        };

        match (self.state, d_ok) {
            // 任何錯誤 → 急停
            (_, None) => {
                self.state = SafetyState::EmergencyBrake;
            }
            // 距離小於等於門檻 → 急停
            (_, Some(d)) if d <= self.threshold_m => {
                self.state = SafetyState::EmergencyBrake;
            }
            // 自動解除條件：距離 > threshold + hysteresis → 回到 Run
            // 包含 +∞ 的情況（代表非常遠）
            (_, Some(d)) if d > self.threshold_m + self.hysteresis_m => {
                self.state = SafetyState::Run;
            }
            // 已急停但尚未超過 hysteresis → SafeStop（保持 0）
            (SafetyState::EmergencyBrake, Some(_)) => {
                self.state = SafetyState::SafeStop;
            }
            _ => {}
        }
        self.state
    }

    /// v0：EmergencyBrake/SafeStop 時速度強制 0；Run 不限速
    pub fn clamp_speed(&self, v_cmd: f32) -> f32 {
        match self.state {
            SafetyState::Run => v_cmd,
            SafetyState::EmergencyBrake | SafetyState::SafeStop => 0.0,
        }
    }

    /// 人為解除（v0 不自動解除）。只有在距離已> threshold + hysteresis 時才回到 Run。
    pub fn reset(&mut self, current_distance_m: Option<f32>) {
        if current_distance_m
            .is_some_and(|d| d.is_finite() && d > self.threshold_m + self.hysteresis_m)
        {
            self.state = SafetyState::Run;
        }
    }

    /// 輔助：用 Option 表示距離（None=錯誤）
    pub fn update_opt(&mut self, distance: Option<f32>) -> SafetyState {
        match distance {
            Some(d) if d.is_nan() => self.update(Err(())),
            Some(d) => self.update(Ok(d)),
            None => self.update(Err(())),
        }
    }
}
