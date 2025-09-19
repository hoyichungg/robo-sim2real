use crate::hal::Telemetry;

// 最小 Telemetry 實作（先印到 stdout）
pub struct StdoutTelemetry;
impl Telemetry for StdoutTelemetry {
    fn record(&mut self, ts_s: f64, key: &str, value: &str) {
        println!("[{ts_s:.3}] {key}={value}");
    }
}
