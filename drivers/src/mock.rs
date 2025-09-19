use r2_core::hal::{DistanceSensor, Motor};

pub struct MockMotor;
impl Motor for MockMotor {
    fn set_wheel_speeds(&mut self, left: f32, right: f32) -> Result<(), String> {
        println!("[MockMotor] left={:.2}, right={:.2}", left, right);
        Ok(())
    }
}

pub struct MockSensor;
impl DistanceSensor for MockSensor {
    fn distance_m(&mut self) -> Result<f32, String> {
        Ok(1.0) // always 1 meter
    }
}
