// 最小占位：PID 與狀態機型別
#[derive(Default)]
pub struct Pid {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub i: f32,
    pub prev_e: f32,
}
impl Pid {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            ..Default::default()
        }
    }
    pub fn step(&mut self, target: f32, measured: f32, dt: f32) -> f32 {
        let e = target - measured;
        self.i += e * dt;
        let d = if dt > 0.0 {
            (e - self.prev_e) / dt
        } else {
            0.0
        };
        self.prev_e = e;
        self.kp * e + self.ki * self.i + self.kd * d
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyState {
    Run,
    EmergencyBrake,
    SafeStop,
}
