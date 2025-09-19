#[derive(Debug, Clone)]
pub struct Pid {
    pub kp: f32, // 比例增益
    pub ki: f32, // 積分增益
    pub kd: f32, // 微分增益

    pub i: f32,      // 積分暫存器
    pub prev_e: f32, // 上一次誤差（給 D 用）

    pub out_min: f32, // 輸出下限（防止過大指令）
    pub out_max: f32, // 輸出上限
    pub i_min: f32,   // 積分下限（防 wind-up）
    pub i_max: f32,   // 積分上限
}

impl Pid {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            i: 0.0,
            prev_e: 0.0,
            out_min: f32::NEG_INFINITY,
            out_max: f32::INFINITY,
            i_min: f32::NEG_INFINITY,
            i_max: f32::INFINITY,
        }
    }

    pub fn with_output_limits(mut self, out_min: f32, out_max: f32) -> Self {
        self.out_min = out_min.min(out_max);
        self.out_max = out_max.max(out_min);
        self
    }

    pub fn with_integral_limits(mut self, i_min: f32, i_max: f32) -> Self {
        self.i_min = i_min.min(i_max);
        self.i_max = i_max.max(i_min);
        self
    }

    pub fn reset(&mut self) {
        self.i = 0.0;
        self.prev_e = 0.0;
    }

    pub fn step(&mut self, target: f32, measured: f32, dt: f32) -> f32 {
        let e = target - measured;

        self.i += e * dt * self.ki;
        if self.i > self.i_max {
            self.i = self.i_max;
        }
        if self.i < self.i_min {
            self.i = self.i_min;
        }

        let d = if dt > 0.0 {
            self.kd * (e - self.prev_e) / dt
        } else {
            0.0
        };
        self.prev_e = e;

        let p = self.kp * e;
        let mut u = p + self.i + d;

        if u > self.out_max {
            u = self.out_max;
        }
        if u < self.out_min {
            u = self.out_min;
        }

        u
    }

    // 呼叫fn step這個方法：
    //  1.	算誤差 e = target - measured。
    //  2.	更新積分 i += e * dt * ki，並強制在 [i_min, i_max] 之間。
    //  3.	算微分 d = kd * (e - prev_e) / dt，用誤差變化量。
    //  4.	算比例 p = kp * e。
    //  5.	合成輸出 u = p + i + d，再強制在 [out_min, out_max] 之間。
    //  6.	回傳這個 u（通常會是馬達速度命令）。
}
