use r2_core::hal::{DistanceSensor, Motor};

// 先做占位 stub，之後再接 rppal
pub struct RpiMotorStub;
impl Motor for RpiMotorStub {
    fn set_wheel_speeds(&mut self, _left: f32, _right: f32) -> Result<(), String> {
        // TODO: 實作 GPIO/PWM
        Ok(())
    }
}

pub struct RpiDistanceStub;
impl DistanceSensor for RpiDistanceStub {
    fn distance_m(&mut self) -> Result<f32, String> {
        // TODO: 讀 I2C 超音波/ToF
        Ok(0.8)
    }
}
