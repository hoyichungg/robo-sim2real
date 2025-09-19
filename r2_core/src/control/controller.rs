use crate::control::pid::Pid;
use crate::control::safety::{FailSafe, SafetyState};

/// 差速運動學（先只用線速度；角速度之後加）
#[derive(Debug, Clone, Copy)]
pub struct DifferentialKinematics {
    pub wheel_base_m: f32,
}
impl DifferentialKinematics {
    pub fn to_wheel_speeds(&self, v_mps: f32, w_rps: f32) -> (f32, f32) {
        let l = v_mps - 0.5 * w_rps * self.wheel_base_m;
        let r = v_mps + 0.5 * w_rps * self.wheel_base_m;
        (l, r)
    }
}

#[derive(Debug)]
pub struct Controller {
    pub pid_v: Pid,
    pub kin: DifferentialKinematics,
    pub safety: FailSafe,
    /// 量測速度（v0 先用回授估計：上一輪命令的低通）
    v_meas: f32,
}

impl Controller {
    pub fn new(pid_v: Pid, kin: DifferentialKinematics, safety: FailSafe) -> Self {
        Self {
            pid_v,
            kin,
            safety,
            v_meas: 0.0,
        }
    }

    /// 單步：輸入「期望線速度」與距離讀值，輸出左右輪速度命令
    pub fn tick(
        &mut self,
        desired_v: f32,
        dt_s: f32,
        distance_m: Result<f32, ()>,
    ) -> ((f32, f32), SafetyState) {
        // 先更新安全狀態
        let st = self.safety.update(distance_m);

        // PID 估算：v_meas 用一個簡單低通當作回授（之後可換真輪速）
        let alpha = 0.2_f32;
        // 先計算未安全夾持的 u，待會再 clamp
        let u_raw = self.pid_v.step(desired_v, self.v_meas, dt_s);

        // 依安全狀態夾持線速度
        let v_cmd = self.safety.clamp_speed(u_raw);

        // 更新偵測到的回授（低通逼近命令），模擬系統惰性
        self.v_meas = (1.0 - alpha) * self.v_meas + alpha * v_cmd;

        // 目前先不做角速度
        let (l, r) = self.kin.to_wheel_speeds(v_cmd, 0.0);
        ((l, r), st)
    }
}
