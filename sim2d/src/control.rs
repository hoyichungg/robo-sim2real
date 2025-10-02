use bevy::prelude::*;
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;
use r2_core::control::adaptive::map_gain;

use crate::components::{Car, Heading, Velocity};
use crate::config::{desired_speed, RuntimeCfg};
use crate::resources::{DistanceSense, SimClock, TelemetryWriter};

/// 控制器內部狀態（用 r2_core::Pid 取代手寫）
pub struct CtrlState {
    pub pid: Pid,
    pub adapt_gain: f32,
    pub safety: FailSafe,
}

/// 控制一步：讀量測 -> PID -> 自適應增益 -> 指令速度
pub fn control_step(
    mut q: Query<(&mut Velocity, &mut Transform, &Heading), With<Car>>,
    mut writer: ResMut<TelemetryWriter>,
    cfg: Res<RuntimeCfg>,
    clk: Res<SimClock>,
    mut st_opt: Local<Option<CtrlState>>,
    distance: Res<DistanceSense>,
) {
    // 初始化一次
    if st_opt.is_none() {
        *st_opt = Some(CtrlState {
            pid: Pid::new(cfg.kp, cfg.ki, cfg.kd)
                .with_output_limits(-1.0, 1.0)
                .with_integral_limits(-0.5, 0.5),
            adapt_gain: 1.0,
            safety: FailSafe::new(cfg.threshold, cfg.hysteresis),
        });
    }
    let st = st_opt.as_mut().unwrap();

    // 更新 FailSafe 狀態
    let fs_state = st.safety.update_opt(Some(distance.0));

    // 量測速度
    let v_meas = q.single().0.v;

    // 期望速度（支援 const/step/sin）
    let v_des = desired_speed(&cfg, clk.t);

    // PID 計算
    let mut u = st.pid.step(v_des, v_meas, clk.dt);

    // 自適應增益
    if cfg.adaptive {
        // 以「當下目標」計算誤差大小，避免使用常數 desired_v 導致估計偏差
        let ae = (v_des - v_meas).abs();
        let gain = map_gain(ae, cfg.e_small, cfg.e_large, cfg.gain_min, cfg.gain_max);
        st.adapt_gain = gain;
        u *= gain;
    } else {
        st.adapt_gain = 1.0;
    }

    // FailSafe 最終裁切
    u = st.safety.clamp_speed(u);

    // 寫回速度
    let (mut vel, _, _) = q.single_mut();
    vel.v = u;

    // Telemetry 輸出（與平台 CSV 對齊）
    let left = u;
    let right = u;
    let dist = distance.0;
    let state_str = format!("{:?}", fs_state);
    let err = v_des - v_meas;
    let meas_left = f32::NAN; // 模擬版依共識填 NaN
    let meas_right = f32::NAN;
    writer.write(
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
    );
}
