use crate::components::{Car, Heading, Obstacle};
use crate::config::RuntimeCfg;
use crate::resources::DistanceSense;
use bevy::prelude::Gizmos;
use bevy::prelude::*;

/// 單障礙方塊：估一個「車頭方向的前方距離（公尺）」
/// - 使用車頭射線與障礙物 AABB 的最近交點
/// - 射線起點取自「車中心向前移動半個車長」（避免車身與障礙重疊才量到 0）
/// - 將世界座標距離（像素）轉成公尺：除以 `cfg.px_per_m`
pub fn sense_distance(
    mut dist: ResMut<DistanceSense>,
    q_car: Query<(&Transform, &Heading, &Sprite), With<Car>>,
    q_obs: Query<(&Transform, &Sprite), With<Obstacle>>,
    cfg: Res<RuntimeCfg>,
    mut gizmos: Gizmos,
) {
    let (car_tx, heading, car_sprite) = match q_car.get_single() {
        Ok(v) => v,
        Err(_) => return,
    };
    let fwd = heading.forward();
    if fwd.length_squared() < 1e-6 {
        dist.0 = f32::INFINITY;
        return;
    }

    // 車長（像素）：用 Sprite.custom_size.x 作為長邊（沒有則視為 0）
    let car_len_px = car_sprite.custom_size.map(|s| s.x.abs()).unwrap_or(0.0);
    let ray_origin = car_tx.translation.truncate() + fwd.normalize() * (0.5 * car_len_px);
    let dir = fwd.normalize();

    let mut best_px = f32::INFINITY;
    let mut best_hit = None;
    let mut best_idx: Option<usize> = None;
    let colors = [
        Color::srgb(0.95, 0.30, 0.30),
        Color::srgb(0.30, 0.85, 0.35),
        Color::srgb(0.30, 0.55, 0.95),
        Color::srgb(0.95, 0.70, 0.30),
        Color::srgb(0.70, 0.30, 0.95),
    ];

    for (i, (obs_tx, obs_sprite)) in q_obs.iter().enumerate() {
        let half = obs_sprite
            .custom_size
            .map(|s| Vec2::new(s.x.abs() * 0.5, s.y.abs() * 0.5))
            .unwrap_or_else(|| Vec2::new(40.0, 40.0));
        let o = obs_tx.translation.truncate();
        let min = o - half;
        let max = o + half;

        // Ray-AABB in 2D（處理接近 0 的分母）
        let inv = Vec2::new(
            if dir.x.abs() < 1e-6 {
                f32::INFINITY
            } else {
                1.0 / dir.x
            },
            if dir.y.abs() < 1e-6 {
                f32::INFINITY
            } else {
                1.0 / dir.y
            },
        );
        let t1 = (min - ray_origin) * inv;
        let t2 = (max - ray_origin) * inv;
        let tmin = f32::min(t1.x, t2.x).max(f32::min(t1.y, t2.y));
        let tmax = f32::max(t1.x, t2.x).min(f32::max(t1.y, t2.y));

        if tmax >= tmin && tmax > 0.0 {
            let t_hit = if tmin > 0.0 { tmin } else { tmax };
            let d_px = t_hit.max(0.0);
            if d_px < best_px {
                best_px = d_px;
                best_hit = Some(ray_origin + dir * d_px);
                best_idx = Some(i);
            }
        }

        // 畫出此障礙的 AABB
        let col = colors[i % colors.len()];
        let p1 = min;
        let p2 = Vec2::new(max.x, min.y);
        let p3 = max;
        let p4 = Vec2::new(min.x, max.y);
        gizmos.line_2d(p1, p2, col);
        gizmos.line_2d(p2, p3, col);
        gizmos.line_2d(p3, p4, col);
        gizmos.line_2d(p4, p1, col);
    }

    // 轉換成公尺並加安全緩衝：扣掉 10% 車長
    let margin_m = cfg.control.margin_ratio() * (car_len_px / cfg.px_per_m);
    let d_m = if best_px.is_finite() {
        (best_px / cfg.px_per_m).max(0.0)
    } else {
        f32::INFINITY
    };
    dist.0 = if d_m.is_finite() {
        (d_m - margin_m).max(0.0)
    } else {
        f32::INFINITY
    };

    // Gizmos：畫出射線與命中點（橘線/圓）。若沒命中，畫短射線指示方向。
    let line_len = if best_px.is_finite() {
        best_px
    } else {
        // 依 threshold 動態設定射線長度（像素）
        (cfg.control.failsafe.threshold_m * cfg.px_per_m + car_len_px).max(60.0)
    };
    let a = ray_origin;
    let b = ray_origin + dir * line_len;
    // 命中時高亮該障礙顏色；否則用橘線指示方向
    if let Some(idx) = best_idx {
        gizmos.line_2d(a, b, colors[idx % colors.len()]);
    } else {
        gizmos.line_2d(a, b, Color::srgb(1.0, 0.27, 0.0));
    }
    if let Some(hit) = best_hit {
        gizmos.circle_2d(hit, 5.0, Color::srgb(1.0, 1.0, 0.0));
    }
}
