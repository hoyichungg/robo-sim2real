use clap::Parser;
use drivers::mock::{MockMotor, MockSensor};
use r2_core::control::controller::{Controller, DifferentialKinematics};
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;
use r2_core::control::telemetry::Telemetry;
use r2_core::hal::{DistanceSensor, Motor};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// Run the minimal control loop (PID + FailSafe) with mock drivers.
#[derive(Parser, Debug)]
#[command(name = "platform_rpi", author, version, about)]
struct Args {
    /// Desired linear speed (m/s)
    #[arg(short = 'v', long, default_value_t = 0.6)]
    desired_v: f32,

    /// Loop frequency (Hz)
    #[arg(long, default_value_t = 50.0)]
    hz: f32,

    /// Run duration (seconds)
    #[arg(long, default_value_t = 10.0)]
    seconds: f32,

    /// Fail-safe threshold distance (m)
    #[arg(long, default_value_t = 0.25)]
    threshold: f32,

    /// Fail-safe hysteresis margin (m) for manual reset
    #[arg(long, default_value_t = 0.05)]
    hysteresis: f32,

    /// Output CSV path (default: telemetry.csv in CWD)
    #[arg(long)]
    csv: Option<PathBuf>,

    /// Suppress per-tick stdout motor prints from MockMotor
    #[arg(long, default_value_t = false)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let mut motor = MockMotor;
    let mut sensor = MockSensor::default();

    let pid = Pid::new(1.0, 0.5, 0.05)
        .with_output_limits(-1.0, 1.0)
        .with_integral_limits(-0.5, 0.5);
    let kin = DifferentialKinematics { wheel_base_m: 0.22 };
    let safety = FailSafe::new(args.threshold, args.hysteresis);
    let mut ctrl = Controller::new(pid, kin, safety);

    let hz = args.hz.max(1.0);
    let dt = Duration::from_secs_f32(1.0 / hz);
    let desired_v = args.desired_v;

    let t0 = Instant::now();
    let mut last = t0;
    let mut log: Vec<Telemetry> = Vec::new();

    let steps = (hz * args.seconds).round() as usize;
    for _ in 0..steps {
        let now = Instant::now();
        let dt_s = (now - last).as_secs_f32().clamp(0.0, 0.05);
        last = now;

        let t_sec = (now - t0).as_secs_f32();

        let dist_val = sensor.distance_m().map_err(|_| ());
        let ((l, r), st) = ctrl.tick(desired_v, dt_s, dist_val);

        let dist_dbg = dist_val.unwrap_or(f32::NAN);
        if let Err(e) = motor.set_wheel_speeds(l, r) {
            eprintln!("motor error: {e}");
        }
        if !args.quiet {
            println!(
                "t={t_sec:5.2}s dt={dt_s:.3}s d={dist_dbg:.2} v_des={desired_v:.2} -> (L={l:.2}, R={r:.2}) state={st:?}"
            );
        }

        log.push(Telemetry {
            t: t_sec,
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
    let csv_path = args.csv.unwrap_or_else(|| PathBuf::from("telemetry.csv"));
    let mut file = File::create(&csv_path).expect("cannot create telemetry csv");
    writeln!(file, "t,dt,desired_v,left,right,distance,state").unwrap();
    for row in log {
        writeln!(
            file,
            "{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},{}",
            row.t, row.dt, row.desired_v, row.left, row.right, row.distance, row.state
        )
        .unwrap();
    }
    eprintln!("Telemetry saved to {}", csv_path.display());
}

// # 預設：v=0.6 m/s, hz=50, seconds=10, threshold=0.25, hysteresis=0.05
// cargo run -p platform_rpi

// # 調快速度 & 門檻
// cargo run -p platform_rpi -- -v 0.8 --threshold 0.3

// # 改頻率、時長、輸出檔名，且安靜模式
// cargo run -p platform_rpi -- --hz 100 --seconds 12 --csv out.csv --quiet
