use r2_core::config::drivers::{DistanceBackend, DriverConfig, MotorBackend};
use r2_core::hal::{DistanceSensor, Motor};

use crate::mock::{MockMotor, MockSensor};

#[cfg(feature = "rpi")]
use crate::rpi::{RpiDistance, RpiMotor};
#[cfg(not(feature = "rpi"))]
use crate::rpi::{RpiDistanceStub, RpiMotorStub};

type MotorHandle = Box<dyn Motor>;
type DistanceHandle = Box<dyn DistanceSensor>;
type DriverPair = (MotorHandle, DistanceHandle);

pub struct DriverFactory;

impl DriverFactory {
    pub fn create_motor(cfg: &MotorBackend) -> Result<MotorHandle, String> {
        match cfg {
            MotorBackend::Mock => Ok(Box::new(MockMotor) as Box<dyn Motor>),
            MotorBackend::Bench(..) => {
                // bench 模式仍沿用 mock motor，實際 bench 模型由呼叫端模擬
                Ok(Box::new(MockMotor) as Box<dyn Motor>)
            }
            MotorBackend::Rpi(rpi_cfg) => {
                #[cfg(feature = "rpi")]
                {
                    RpiMotor::new(*rpi_cfg).map(|m| Box::new(m) as Box<dyn Motor>)
                }
                #[cfg(not(feature = "rpi"))]
                {
                    let _ = rpi_cfg;
                    Ok(Box::new(RpiMotorStub) as Box<dyn Motor>)
                }
            }
        }
    }

    pub fn create_distance(cfg: &DistanceBackend) -> Result<DistanceHandle, String> {
        match cfg {
            DistanceBackend::Mock => Ok(Box::new(MockSensor::default()) as Box<dyn DistanceSensor>),
            DistanceBackend::RpiHcsr04 {
                trig_gpio,
                echo_gpio,
            } => {
                #[cfg(feature = "rpi")]
                {
                    RpiDistance::new(*trig_gpio, *echo_gpio)
                        .map(|d| Box::new(d) as Box<dyn DistanceSensor>)
                }
                #[cfg(not(feature = "rpi"))]
                {
                    let _ = (trig_gpio, echo_gpio);
                    Ok(Box::new(RpiDistanceStub) as Box<dyn DistanceSensor>)
                }
            }
        }
    }

    pub fn create_all(cfg: &DriverConfig) -> Result<DriverPair, String> {
        let motor = Self::create_motor(&cfg.motor)?;
        let distance = Self::create_distance(&cfg.distance)?;
        Ok((motor, distance))
    }
}
