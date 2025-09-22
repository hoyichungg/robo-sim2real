use drivers::mock::{MockMotor, MockSensor};
use r2_core::control::controller::{Controller, DifferentialKinematics};
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;
use r2_core::control::telemetry::Telemetry;
use r2_core::hal::{DistanceSensor, Motor};
use std::fs::File;
use std::io::Write;
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

    let mut last = Instant::now();
    let mut log: Vec<Telemetry> = Vec::new();

    let seconds = 10.0;
    for _ in 0..(hz as usize * seconds as usize) {
        let now = Instant::now();
        let dt_s = (now - last).as_secs_f32().clamp(0.0, 0.05);
        last = now;

        let dist_val = sensor.distance_m().map_err(|_| ());
        let ((l, r), st) = ctrl.tick(desired_v, dt_s, dist_val);

        let dist_dbg = dist_val.unwrap_or(f32::NAN);
        let _ = motor.set_wheel_speeds(l, r);

        log.push(Telemetry {
            t: last.elapsed().as_secs_f32(),
            dt: dt_s,
            desired_v,
            left: l,
            right: r,
            distance: dist_dbg,
            state: format!("{:?}", st),
        });

        thread::sleep(dt);
    }

    // 寫出 CSV
    let mut file = File::create("telemetry.csv").expect("cannot create file");
    writeln!(file, "t,dt,desired_v,left,right,distance,state").unwrap();
    for row in log {
        writeln!(
            file,
            "{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},{}",
            row.t, row.dt, row.desired_v, row.left, row.right, row.distance, row.state
        )
        .unwrap();
    }

    println!("Telemetry saved to telemetry.csv");
}
