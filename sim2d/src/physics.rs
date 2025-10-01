use crate::components::{Car, Heading, Velocity};
use crate::config::RuntimeCfg;
use crate::resources::SimClock;
use bevy::prelude::*;

/// 用一階系統把控制輸出（左右合成）轉成測得速度 v_meas，再整步更新位置。
/// 將控制速度 vel.v 用一階系統濾成 v_meas，再積分到位置
pub fn integrate_kinematics(
    mut q: Query<(&mut Velocity, &mut Transform, &Heading), With<Car>>,
    cfg: Res<RuntimeCfg>,
    clk: Res<SimClock>,
) {
    let Ok((mut vel, mut tf, heading)) = q.get_single_mut() else {
        return;
    };

    // --- 一階系統：命令 → 量測 ---
    let alpha = 1.0 - (-clk.dt / cfg.tau).exp();
    let v_cmd = cfg.plant_gain * vel.v; // 命令經過 plant_gain
    let v_meas = vel.v + alpha * (v_cmd - vel.v);
    vel.v = v_meas;

    // --- 位置更新 ---
    // ⚠️ 改用 heading.0；若你有 forward()，也請確認它真的回傳的是「單位向量」
    let fwd = heading.0.normalize_or_zero(); // Vec2
    tf.translation.x += fwd.x * v_meas * clk.dt * cfg.px_per_m;
    tf.translation.y += fwd.y * v_meas * clk.dt * cfg.px_per_m;

    // （移除除錯輸出，避免汙染 stdout）
}
