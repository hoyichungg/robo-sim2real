#[derive(Debug)]
pub struct Telemetry {
    pub t: f32,          // 時間戳 (秒)
    pub dt: f32,         // 這次迴圈時間 (秒)
    pub desired_v: f32,  // 目標速度
    pub left: f32,       // 左輪輸出
    pub right: f32,      // 右輪輸出
    pub distance: f32,   // 感測距離
    pub state: String,   // FailSafe 狀態
    
    pub meas_left: f32,  // bench 模式產生的左輪「量測速度」
    pub meas_right: f32, // bench 模式產生的右輪「量測速度」
}
