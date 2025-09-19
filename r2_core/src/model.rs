// 指令與運動學占位
pub struct Command {
    pub linear_mps: f32,
    pub angular_rps: f32, // v0 先不用
    pub brake: bool,
}

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
