use drivers::mock::{MockMotor, MockSensor};
use r2_core::control::controller::{Controller, DifferentialKinematics};
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;
use r2_core::hal::{DistanceSensor, Motor};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let mut motor = MockMotor;
    let mut sensor = MockSensor::new();

    let pid = Pid::new(1.0, 0.5, 0.05)
        .with_output_limits(-1.0, 1.0)
        .with_integral_limits(-0.5, 0.5);
    let kin = DifferentialKinematics { wheel_base_m: 0.22 };
    let safety = FailSafe::new(0.25, 0.05);
    let mut ctrl = Controller::new(pid, kin, safety);

    let hz = 50.0;
    let dt = Duration::from_secs_f32(1.0 / hz as f32);
    let desired_v = 0.6_f32;
    let seconds = 10.0; // 模擬總時長

    let mut last = Instant::now();
    for _ in 0..(hz as usize * seconds as usize) {
        // 計算 dt 並夾住避免抖動
        let now = Instant::now();
        let mut dt_s = (now - last).as_secs_f32();
        last = now;
        dt_s = dt_s.clamp(0.0, 0.05);

        // 讀取距離
        let dist_val = sensor.distance_m().map_err(|_| ());
        let ((l, r), st) = ctrl.tick(desired_v, dt_s, dist_val);

        // 印出 debug 資訊（包含距離）
        let dist_dbg = dist_val.unwrap_or(f32::NAN);
        println!(
            "dt={dt_s:.3}s d={dist_dbg:.2} v_des={desired_v:.2} -> (L={l:.2}, R={r:.2}) state={st:?}"
        );

        // 設定馬達
        if let Err(e) = motor.set_wheel_speeds(l, r) {
            eprintln!("motor error: {e}");
        }

        thread::sleep(dt);
    }

    println!("Done.");
}
