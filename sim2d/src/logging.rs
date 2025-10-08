use crate::resources::{SimClock, TelemetryWriter};
use bevy::prelude::*;

/// 依你自己的 Writer 設計：這裡先保留「每個 fixed step flush 一次」的骨架
pub fn flush_telemetry(
    mut writer: ResMut<TelemetryWriter>,
    clk: Res<SimClock>,
    mut accum: Local<f32>,
) {
    *accum += clk.dt;
    if *accum >= 0.5 {
        if let Err(err) = writer.flush() {
            eprintln!("telemetry flush failed: {err}");
        }
        *accum = 0.0;
    }
}
