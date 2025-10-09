use bevy::prelude::*;
use r2_core::control::telemetry::{TelemetrySample, TelemetrySink};
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

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl TelemetrySink for TelemetryWriter {
    fn record(&mut self, sample: &TelemetrySample) -> std::io::Result<()> {
        writeln!(
            self.file,
            "{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{:.3},{:.3},{:.3},{:.3}",
            sample.t,
            sample.dt,
            sample.desired_v,
            sample.left,
            sample.right,
            sample.distance,
            sample.state,
            sample.meas_left,
            sample.meas_right,
            sample.err,
            sample.adapt_gain
        )
    }

    fn flush(&mut self) -> std::io::Result<()> {
        TelemetryWriter::flush(self)
    }
}

/// 固定步進時鐘
#[derive(Resource, Default)]
pub struct SimClock {
    pub t: f32,
    pub dt: f32,
}
