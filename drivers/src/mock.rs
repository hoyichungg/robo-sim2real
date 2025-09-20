use r2_core::hal::{DistanceSensor, Motor};
use std::time::Instant;

pub struct MockMotor;
impl Motor for MockMotor {
    fn set_wheel_speeds(&mut self, left: f32, right: f32) -> Result<(), String> {
        println!("[MockMotor] left={left:.2}, right={right:.2}");
        Ok(())
    }
}

pub struct MockSensor {
    t0: Instant,
}
impl MockSensor {
    pub fn new() -> Self {
        Self { t0: Instant::now() }
    }
}
impl Default for MockSensor {
    fn default() -> Self {
        Self::new()
    }
}
impl DistanceSensor for MockSensor {
    fn distance_m(&mut self) -> Result<f32, String> {
        let t = self.t0.elapsed().as_secs_f32();
        // 0~2s: 固定 1.0m；2~6s 線性下降到 0.1m；之後保持 0.1m
        let d = if t < 2.0 {
            1.0
        } else if t < 6.0 {
            let k = (t - 2.0) / 4.0;
            1.0 * (1.0 - k) + 0.1 * k
        } else {
            0.1
        };
        Ok(d)
    }
}
