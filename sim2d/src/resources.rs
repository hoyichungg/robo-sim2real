use bevy::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct DistanceSense(pub f32); // 目前距離量測（m）

#[derive(Resource)]
pub struct TelemetryWriter {
    file: File,
}
impl TelemetryWriter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).ok();
        }
        let mut f = File::create(path).expect("create sim csv");
        writeln!(
            f,
            "t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain"
        )
        .ok();
        Self { file: f }
    }
    pub fn write(
        &mut self,
        t: f32,
        dt: f32,
        v_des: f32,
        left: f32,
        right: f32,
        dist: f32,
        state: &str,
        meas_left: f32,
        meas_right: f32,
        err: f32,
        gain: f32,
    ) {
        writeln!(
            self.file,
            "{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{:.3},{:.3},{:.3},{:.3}",
            t, dt, v_des, left, right, dist, state, meas_left, meas_right, err, gain
        )
        .ok();
    }
}

/// 固定步進時鐘
#[derive(Resource, Default)]
pub struct SimClock {
    pub t: f32,
    pub dt: f32,
}
