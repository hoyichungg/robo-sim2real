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
        let d_ok = distance.ok().filter(|d| d.is_finite()).map(|d| d.max(0.0)); // clamp 負值

        match (self.state, d_ok) {
            // 任何錯誤 → 急停
            (_, None) => {
                self.state = SafetyState::EmergencyBrake;
            }
            // 距離小於等於門檻 → 急停
            (_, Some(d)) if d <= self.threshold_m => {
                self.state = SafetyState::EmergencyBrake;
            }
            // 已急停且距離回升到安全區，轉為 SafeStop（保持 0）
            (SafetyState::EmergencyBrake, Some(d)) if d > self.threshold_m => {
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
        if let Some(d) = current_distance_m {
            if d.is_finite() && d > self.threshold_m + self.hysteresis_m {
                self.state = SafetyState::Run;
            }
        }
    }

    /// 輔助：用 Option 表示距離（None=錯誤）
    pub fn update_opt(&mut self, distance: Option<f32>) -> SafetyState {
        match distance {
            Some(d) if d.is_finite() => self.update(Ok(d)),
            _ => self.update(Err(())),
        }
    }
}
