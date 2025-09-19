#[derive(Debug, Clone)]
pub struct Pid {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub i: f32,
    pub prev_e: f32,
    pub out_min: f32,
    pub out_max: f32,
    pub i_min: f32,
    pub i_max: f32,
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
}
