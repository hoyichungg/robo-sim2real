use bevy::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Resource, Default)]
pub struct DistanceSense(pub f32); // 目前距離量測（m）

#[derive(Resource)]
pub struct TelemetryWriter {
    file: BufWriter<File>,
}
impl TelemetryWriter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).ok();
        }
        let file = File::create(path).expect("create sim csv");
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain"
        )
        .expect("write sim csv header");
        Self { file: writer }
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
    ) -> std::io::Result<()> {
        writeln!(
            self.file,
            "{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{:.3},{:.3},{:.3},{:.3}",
            t, dt, v_des, left, right, dist, state, meas_left, meas_right, err, gain
        )?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// 固定步進時鐘
#[derive(Resource, Default)]
pub struct SimClock {
    pub t: f32,
    pub dt: f32,
}
