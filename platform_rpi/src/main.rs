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
use std::time::{Duration, Instant};

mod profile;
use profile::{ProfileParams, VProfile};

/// Run the minimal control loop (PID + FailSafe) with mock drivers.
#[derive(Parser, Debug)]
#[command(name = "platform_rpi", author, version, about)]
struct Args {
    /// Desired linear speed (m/s)
    #[arg(short = 'v', long, default_value_t = 0.6)]
    desired_v: f32,

    /// PID gains
    #[arg(long, default_value_t = 1.0)]
    kp: f32,
    #[arg(long, default_value_t = 0.5)]
    ki: f32,
    #[arg(long, default_value_t = 0.05)]
    kd: f32,

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

    /// Velocity profile type
    #[arg(long, value_enum)]
    v_profile: Option<VProfile>,

    /// Step profile: step time (s)
    #[arg(long)]
    step_at: Option<f32>,

    /// Sin profile: amplitude
    #[arg(long)]
    sin_amp: Option<f32>,
    /// Sin profile: frequency (Hz)
    #[arg(long)]
    sin_freq: Option<f32>,
    /// Sin profile: bias (offset)
    #[arg(long)]
    sin_bias: Option<f32>,

    /// 啟用 bench 模擬模式（內部一階馬達模型）
    #[arg(long, default_value_t = false)]
    bench: bool,
    /// bench 模型時間常數 τ (s)
    #[arg(long, default_value_t = 0.8)]
    bench_tau: f32,
    /// bench 模型增益 (u→v)
    #[arg(long, default_value_t = 0.6)]
    bench_gain: f32,
}

fn main() {
    let args = Args::parse();

    // 把 Step/Sin 參數集中
    let params = ProfileParams {
        step_at: args.step_at.unwrap_or(1.0),
        sin_amp: args.sin_amp.unwrap_or(0.3),
        sin_freq: args.sin_freq.unwrap_or(0.2),
        sin_bias: args.sin_bias.unwrap_or(0.4),
    };

    let mut motor = MockMotor;
    let mut sensor = MockSensor::default();

    let pid = Pid::new(args.kp, args.ki, args.kd)
        .with_output_limits(-1.0, 1.0)
        .with_integral_limits(-0.5, 0.5);
    let kin = DifferentialKinematics { wheel_base_m: 0.22 };
    let safety = FailSafe::new(args.threshold, args.hysteresis);
    let mut ctrl = Controller::new(pid, kin, safety);

    let hz = args.hz.max(1.0);
    let dt = Duration::from_secs_f32(1.0 / hz);
    let t0 = Instant::now();
    let mut last = t0;
    let mut log: Vec<Telemetry> = Vec::new();

    // bench 模式的內部狀態
    let mut v_meas_l = 0.0f32;
    let mut v_meas_r = 0.0f32;
    let tau = args.bench_tau.max(1e-3); // 避免 0
    let gain = args.bench_gain;

    let steps = (hz * args.seconds).round() as usize;
    for _ in 0..steps {
        let now = Instant::now();
        let dt_s = (now - last).as_secs_f32().clamp(0.0, 0.05);
        last = now;
        let t_sec = (now - t0).as_secs_f32();

        // 期望速度（從模組計算）
        let desired_v = profile::desired_v(args.v_profile, params, args.desired_v, t_sec);

        // 感測 + 控制
        let dist_val = sensor.distance_m().map_err(|_| ());
        let ((l, r), st) = ctrl.tick(desired_v, dt_s, dist_val);

        // bench 模式：用控制輸出驅動內部一階模型
        if args.bench {
            let alpha = 1.0 - (-dt_s / tau).exp();
            let v_cmd_l = gain * l;
            let v_cmd_r = gain * r;
            v_meas_l += alpha * (v_cmd_l - v_meas_l);
            v_meas_r += alpha * (v_cmd_r - v_meas_r);
        }

        // 驅動馬達（mock）
        let dist_dbg = dist_val.unwrap_or(f32::NAN);
        if let Err(e) = motor.set_wheel_speeds(l, r) {
            eprintln!("motor error: {e}");
        }
        if !args.quiet {
            println!(
                "t={t_sec:5.2}s dt={dt_s:.3}s d={dist_dbg:.2} v_des={desired_v:.2} \
                -> (L={l:.2}, R={r:.2}) state={st:?} meas=({:.2},{:.2})",
                v_meas_l, v_meas_r
            );
        }

        // 記錄
        log.push(Telemetry {
            t: t_sec,
            dt: dt_s,
            desired_v,
            left: l,
            right: r,
            distance: dist_dbg,
            state: format!("{:?}", st),
            meas_left: if args.bench { v_meas_l } else { f32::NAN },
            meas_right: if args.bench { v_meas_r } else { f32::NAN },
        });

        std::thread::sleep(dt);
    }

    // 輸出 CSV
    let csv_path = args.csv.unwrap_or_else(|| PathBuf::from("telemetry.csv"));
    let mut file = File::create(&csv_path).expect("cannot create telemetry csv");
    writeln!(
        file,
        "t,dt,desired_v,left,right,distance,state,meas_left,meas_right"
    )
    .unwrap();
    for row in log {
        let ml = if args.bench { row.meas_left } else { f32::NAN };
        let mr = if args.bench { row.meas_right } else { f32::NAN };
        writeln!(
            file,
            "{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},{},{:.3},{:.3}",
            row.t, row.dt, row.desired_v, row.left, row.right, row.distance, row.state, ml, mr
        )
        .unwrap();
    }
    eprintln!("Telemetry saved to {}", csv_path.display());
}
