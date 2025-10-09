use r2_core::config::drivers::{DistanceBackend, DriverConfig, MotorBackend};
use r2_core::hal::{DistanceSensor, Motor};

use crate::mock::{MockMotor, MockSensor};

#[cfg(feature = "rpi")]
use crate::rpi::{RpiDistance, RpiMotor};
#[cfg(not(feature = "rpi"))]
use crate::rpi::{RpiDistanceStub, RpiMotorStub};

pub type MotorHandle = Box<dyn Motor>;
pub type DistanceHandle = Box<dyn DistanceSensor>;

use std::any::Any;
use std::collections::HashMap;

/// 可擴充的 Driver 集合。預設提供 motor/distance，也能透過 extras 保留自訂裝置。
pub struct DriverSet {
    motor: Option<MotorHandle>,
    distance: Option<DistanceHandle>,
    extras: HashMap<&'static str, Box<dyn Any + Send>>,
}

impl DriverSet {
    pub fn new() -> Self {
        Self {
            motor: None,
            distance: None,
            extras: HashMap::new(),
        }
    }

    pub fn set_motor(&mut self, motor: MotorHandle) {
        self.motor = Some(motor);
    }

    pub fn set_distance(&mut self, distance: DistanceHandle) {
        self.distance = Some(distance);
    }

    pub fn insert_extra(&mut self, key: &'static str, handle: Box<dyn Any + Send>) {
        self.extras.insert(key, handle);
    }

    pub fn extra<T: 'static>(&self, key: &str) -> Option<&T> {
        self.extras
            .get(key)
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    pub fn into_handles(self) -> Result<DriverHandles, String> {
        let motor = self
            .motor
            .ok_or_else(|| "driver factory did not produce a motor handle".to_string())?;
        let distance = self
            .distance
            .ok_or_else(|| "driver factory did not produce a distance handle".to_string())?;
        Ok(DriverHandles { motor, distance })
    }
}

impl Default for DriverSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Named collection of driver handles returned by the factory.
pub struct DriverHandles {
    pub motor: MotorHandle,
    pub distance: DistanceHandle,
}

pub trait DeviceBuilder: Send + Sync {
    fn build(&self, cfg: &DriverConfig, set: &mut DriverSet) -> Result<(), String>;
}

struct MotorBuilder;

impl DeviceBuilder for MotorBuilder {
    fn build(&self, cfg: &DriverConfig, set: &mut DriverSet) -> Result<(), String> {
        let handle: MotorHandle = match &cfg.motor {
            MotorBackend::Mock => Box::new(MockMotor),
            MotorBackend::Bench(bench_cfg) => {
                set.insert_extra("bench_motor", Box::new(*bench_cfg));
                Box::new(MockMotor)
            }
            MotorBackend::Rpi(rpi_cfg) => {
                #[cfg(feature = "rpi")]
                {
                    Box::new(RpiMotor::new(*rpi_cfg)?)
                }
                #[cfg(not(feature = "rpi"))]
                {
                    let _ = rpi_cfg;
                    Box::new(RpiMotorStub)
                }
            }
        };
        set.set_motor(handle);
        Ok(())
    }
}

struct DistanceBuilder;

impl DeviceBuilder for DistanceBuilder {
    fn build(&self, cfg: &DriverConfig, set: &mut DriverSet) -> Result<(), String> {
        let handle: DistanceHandle = match &cfg.distance {
            DistanceBackend::Mock => Box::new(MockSensor::default()),
            DistanceBackend::RpiHcsr04 {
                trig_gpio,
                echo_gpio,
            } => {
                #[cfg(feature = "rpi")]
                {
                    Box::new(RpiDistance::new(*trig_gpio, *echo_gpio)?)
                }
                #[cfg(not(feature = "rpi"))]
                {
                    let _ = (trig_gpio, echo_gpio);
                    Box::new(RpiDistanceStub)
                }
            }
        };
        set.set_distance(handle);
        Ok(())
    }
}

pub struct DriverFactory {
    builders: Vec<Box<dyn DeviceBuilder>>,
}

impl DriverFactory {
    pub fn new() -> Self {
        Self {
            builders: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut factory = Self::new();
        factory.register(MotorBuilder);
        factory.register(DistanceBuilder);
        factory
    }

    pub fn register<B>(&mut self, builder: B)
    where
        B: DeviceBuilder + 'static,
    {
        self.builders.push(Box::new(builder));
    }

    pub fn build(&self, cfg: &DriverConfig) -> Result<DriverSet, String> {
        let mut set = DriverSet::default();
        for builder in &self.builders {
            builder.build(cfg, &mut set)?;
        }
        Ok(set)
    }

    pub fn create_all(cfg: &DriverConfig) -> Result<DriverHandles, String> {
        Self::with_defaults().build(cfg)?.into_handles()
    }
}

impl Default for DriverFactory {
    fn default() -> Self {
        Self::with_defaults()
    }
}
