use bevy::prelude::*;
use r2_core::config::control::AdaptiveConfig;
use r2_core::control::adaptive::map_gain;
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;

use crate::components::{Car, Velocity};
use crate::config::{desired_speed, RuntimeCfg};
use crate::resources::{DistanceSense, SimClock, TelemetryWriter};

/// 控制器內部狀態（用 r2_core::Pid 取代手寫）
pub struct CtrlState {
    pub pid: Pid,
    pub adapt_gain: f32,
    pub safety: FailSafe,
    pub adaptive_cfg: AdaptiveConfig,
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
            adapt_gain: 1.0,
            safety: control_cfg.build_failsafe(),
            adaptive_cfg: control_cfg.adaptive,
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

    // 自適應增益
    if st.adaptive_cfg.enabled {
        let ae = (v_des - v_meas).abs();
        let gain = map_gain(
            ae,
            st.adaptive_cfg.e_small,
            st.adaptive_cfg.e_large,
            st.adaptive_cfg.gain_min,
            st.adaptive_cfg.gain_max,
        );
        st.adapt_gain = gain;
        u *= gain;
    } else {
        st.adapt_gain = 1.0;
    }

    // FailSafe 最終裁切
    let fs_state = st.safety.update_opt(Some(distance.0));
    u = st.safety.clamp_speed(u);
    vel.cmd = u;

    // Telemetry 輸出（與平台 CSV 對齊）
    let left = u;
    let right = u;
    let dist = distance.0;
    let state_str = format!("{:?}", fs_state);
    let err = v_des - v_meas;
    let meas_left = v_meas; // 模擬 plant 左右輪同速
    let meas_right = v_meas;
    if let Err(err) = writer.write(
        clk.t,
        clk.dt,
        v_des,
        left,
        right,
        dist,
        &state_str,
        meas_left,
        meas_right,
        err,
        st.adapt_gain,
    ) {
        eprintln!("telemetry write failed: {err}");
    }
}
