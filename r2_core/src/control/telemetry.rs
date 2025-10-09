#[derive(Debug, Clone)]
pub struct TelemetrySample {
    pub t: f32,          // 時間戳 (秒)
    pub dt: f32,         // 這次迴圈時間 (秒)
    pub desired_v: f32,  // 目標速度
    pub left: f32,       // 左輪輸出
    pub right: f32,      // 右輪輸出
    pub distance: f32,   // 感測距離
    pub state: String,   // FailSafe 狀態
    pub meas_left: f32,  // 左輪量測速度
    pub meas_right: f32, // 右輪量測速度
    pub err: f32,        // 當下平均速度誤差
    pub adapt_gain: f32, // 自適應輸出增益
}

/// 抽象化的 Telemetry 輸出介面，方便未來換成 CSV / IPC / 其他格式。
pub trait TelemetrySink {
    fn record(&mut self, sample: &TelemetrySample) -> std::io::Result<()>;

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
