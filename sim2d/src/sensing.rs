use crate::components::{Car, Heading, Obstacle};
use crate::resources::DistanceSense;
use bevy::prelude::*;

/// 單障礙方塊：估一個「車頭方向的前方距離」
/// 這裡用「車中心到障礙物 AABB 在車頭射線上的最近交點」的簡化估算。
pub fn sense_distance(
    mut dist: ResMut<DistanceSense>,
    q_car: Query<(&Transform, &Heading), With<Car>>,
    q_obs: Query<&Transform, With<Obstacle>>,
) {
    let (car_tx, heading) = match q_car.get_single() {
        Ok(v) => v,
        Err(_) => return,
    };
    let fwd = heading.forward();
    if fwd.length_squared() < 1e-6 {
        dist.0 = f32::INFINITY;
        return;
    }

    // 取第一個障礙
    if let Ok(obs_tx) = q_obs.get_single() {
        // 假設障礙為軸對齊方塊，大小用 Sprite 的 scale.x/y（簡化）
        let half = Vec2::new(obs_tx.scale.x.abs() * 0.5, obs_tx.scale.y.abs() * 0.5);
        let c = car_tx.translation.truncate();
        let o = obs_tx.translation.truncate();

        // Ray-AABB in 2D
        let dir = fwd.normalize();
        let min = o - half;
        let max = o + half;

        let inv = Vec2::new(1.0 / dir.x.max(1e-6), 1.0 / dir.y.max(1e-6));
        let t1 = (min - c) * inv;
        let t2 = (max - c) * inv;

        let tmin = f32::min(t1.x, t2.x).max(f32::min(t1.y, t2.y));
        let tmax = f32::max(t1.x, t2.x).min(f32::max(t1.y, t2.y));

        let d = if tmax >= tmin && tmax > 0.0 {
            // 最近正向交點
            let t_hit = if tmin > 0.0 { tmin } else { tmax };
            (c + dir * t_hit - c).length()
        } else {
            f32::INFINITY
        };

        dist.0 = d;
    }
}
