/// 自適應增益工具：將 |e| ∈ [e_small, e_large] 線性映射到 [g_min, g_max]，區間外做夾限。
#[inline]
pub fn map_gain(abs_e: f32, e_small: f32, e_large: f32, g_min: f32, g_max: f32) -> f32 {
    if abs_e <= e_small {
        g_min
    } else if abs_e >= e_large {
        g_max
    } else {
        let denom = (e_large - e_small).max(1e-12);
        let r = (abs_e - e_small) / denom;
        g_min + r * (g_max - g_min)
    }
}

#[cfg(test)]
mod tests {
    use super::map_gain;

    #[test]
    fn clamps_below_small() {
        assert!((map_gain(0.0, 0.02, 0.2, 0.6, 1.2) - 0.6).abs() < 1e-6);
        assert!((map_gain(0.02, 0.02, 0.2, 0.6, 1.2) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn clamps_above_large() {
        assert!((map_gain(0.3, 0.02, 0.2, 0.6, 1.2) - 1.2).abs() < 1e-6);
        assert!((map_gain(0.2, 0.02, 0.2, 0.6, 1.2) - 1.2).abs() < 1e-6);
    }

    #[test]
    fn interpolates_linearly() {
        let g = map_gain(0.11, 0.02, 0.2, 0.6, 1.2); // 正中間
        assert!((g - 0.9).abs() < 1e-6);
    }
}