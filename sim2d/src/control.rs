use bevy::prelude::*;
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;

use crate::components::{Car, Velocity};
use crate::config::{desired_speed, RuntimeCfg};
use crate::resources::{DistanceSense, SimClock, TelemetryWriter};
use r2_core::control::adaptive::{DynGainScheduler, GainScheduler};
use r2_core::control::telemetry::{TelemetrySample, TelemetrySink};

/// 控制器內部狀態（用 r2_core::Pid 取代手寫）
pub struct CtrlState {
    pub pid: Pid,
    pub safety: FailSafe,
    pub gain_sched: DynGainScheduler,
    pub adapt_gain: f32,
}

/// 控制一步：讀量測 -> PID -> 自適應增益 -> 指令速度
pub fn control_step(
    mut q: Query<&mut Velocity, With<Car>>,
    mut writer: ResMut<TelemetryWriter>,
    cfg: Res<RuntimeCfg>,
    clk: Res<SimClock>,
    mut st_opt: Local<Option<CtrlState>>,
    distance: Res<DistanceSense>,
) {
    // 初始化一次
    if st_opt.is_none() {
        let control_cfg = cfg.control;
        *st_opt = Some(CtrlState {
            pid: control_cfg.build_pid(),
            safety: control_cfg.build_failsafe(),
            gain_sched: control_cfg.adaptive.build_scheduler(),
            adapt_gain: 1.0,
        });
    }
    let st = st_opt.as_mut().unwrap();
    let mut vel = match q.get_single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };
    let v_meas = vel.meas;

    // 期望速度（支援 const/step/sin）
    let v_des = desired_speed(&cfg, clk.t);

    // PID 計算
    let mut u = st.pid.step(v_des, v_meas, clk.dt);

    // 自適應輸出增益
    let gain = st.gain_sched.update(v_des - v_meas);
    st.adapt_gain = gain;
    u *= gain;

    // FailSafe 最終裁切
    let fs_state = st.safety.update_opt(Some(distance.0));
    u = st.safety.clamp_speed(u);
    vel.cmd = u;

    // Telemetry 輸出（與平台 CSV 對齊）
    let left = u;
    let right = u;
    let dist = distance.0;
    let err = v_des - v_meas;
    let state_str = format!("{:?}", fs_state);
    let sample = TelemetrySample {
        t: clk.t,
        dt: clk.dt,
        desired_v: v_des,
        left,
        right,
        distance: dist,
        state: state_str,
        meas_left: v_meas,  // 模擬 plant 左右輪同速
        meas_right: v_meas, // same as above
        err,
        adapt_gain: st.adapt_gain,
    };

    if let Err(err) = writer.record(&sample) {
        eprintln!("telemetry write failed: {err}");
    }
}
