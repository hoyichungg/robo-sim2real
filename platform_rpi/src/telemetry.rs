use r2_core::control::telemetry::{TelemetrySample, TelemetrySink};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct CsvTelemetrySink {
    writer: BufWriter<File>,
}

impl CsvTelemetrySink {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain"
        )?;
        Ok(Self { writer })
    }
}

impl TelemetrySink for CsvTelemetrySink {
    fn record(&mut self, sample: &TelemetrySample) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},{},{:.3},{:.3},{:.4},{:.3}",
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
        self.writer.flush()
    }
}
