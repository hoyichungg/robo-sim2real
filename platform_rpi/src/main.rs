use clap::Parser;
use drivers::mock::{MockMotor, MockSensor};
use r2_core::config::control::{ControlConfig, ControlOverrides};
use r2_core::control::controller::{Controller, DifferentialKinematics};
use r2_core::hal::{DistanceSensor, Motor};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod profile;
use profile::{ProfileParams, VProfile};

mod adaptive;
use adaptive::map_gain;

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

    /// Output CSV file name (會被寫進 run/ 資料夾)
    #[arg(long)]
    csv: Option<PathBuf>,

    /// Suppress per-tick stdout motor prints from MockMotor
    #[arg(long, default_value_t = false)]
    quiet: bool,

    /// Velocity profile type
    #[arg(long, value_enum, ignore_case = true)]
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

    /// 啟用誤差導向的自適應輸出增益
    #[arg(long, default_value_t = false)]
    adaptive: bool,

    /// |error| ≤ e_small 時使用 gain_min
    #[arg(long, default_value_t = 0.02)]
    e_small: f32,
    /// |error| ≥ e_large 時使用 gain_max
    #[arg(long, default_value_t = 0.20)]
    e_large: f32,

    /// 最小 / 最大輸出增益（線性內插）
    #[arg(long, default_value_t = 0.6)]
    gain_min: f32,
    #[arg(long, default_value_t = 1.2)]
    gain_max: f32,
}

impl Args {
    fn control_overrides(&self) -> ControlOverrides {
        let mut overrides = ControlOverrides::default();
        overrides.pid.kp = Some(self.kp);
        overrides.pid.ki = Some(self.ki);
        overrides.pid.kd = Some(self.kd);
        overrides.failsafe.threshold_m = Some(self.threshold);
        overrides.failsafe.hysteresis_m = Some(self.hysteresis);
        overrides.adaptive.enabled = Some(self.adaptive);
        overrides.adaptive.e_small = Some(self.e_small);
        overrides.adaptive.e_large = Some(self.e_large);
        overrides.adaptive.gain_min = Some(self.gain_min);
        overrides.adaptive.gain_max = Some(self.gain_max);
        overrides
    }
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

    let control_cfg = ControlConfig::default().with_overrides(&args.control_overrides());

    let pid = control_cfg.build_pid();
    let kin = DifferentialKinematics { wheel_base_m: 0.22 };
    let safety = control_cfg.build_failsafe();
    let adaptive_cfg = control_cfg.adaptive;
    let mut ctrl = Controller::new(pid, kin, safety);

    let hz = args.hz.max(1.0);
    let dt = Duration::from_secs_f32(1.0 / hz);
    let t0 = Instant::now();
    let mut last = t0;

    // bench 模式的內部狀態
    let mut v_meas_l = 0.0f32;
    let mut v_meas_r = 0.0f32;
    let tau = args.bench_tau.max(1e-3); // 避免 0
    let gain = args.bench_gain;

    // === 準備 CSV Writer ===
    let csv_path = args
        .csv
        .clone()
        .unwrap_or_else(|| PathBuf::from("run/telemetry.csv"));
    if let Some(parent) = csv_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).expect("failed to create CSV parent dir");
    }
    let file = File::create(&csv_path).expect("cannot create telemetry csv");
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain"
    )
    .expect("write csv header");

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
        let ((l_raw, r_raw), st) = ctrl.tick(desired_v, dt_s, dist_val);

        // bench：用控制輸出驅動一階模型 → 產生「量測速度」
        if args.bench {
            let alpha = 1.0 - (-dt_s / tau).exp();
            let v_cmd_l = gain * l_raw;
            let v_cmd_r = gain * r_raw;
            v_meas_l += alpha * (v_cmd_l - v_meas_l);
            v_meas_r += alpha * (v_cmd_r - v_meas_r);
        }

        // 估測誤差（有 meas 用 meas，否則用簡單近似）
        let v_meas_avg = if args.bench {
            0.5 * (v_meas_l + v_meas_r)
        } else {
            gain * 0.5 * (l_raw + r_raw)
        };
        let ctrl_err = desired_v - v_meas_avg;
        let abs_e = ctrl_err.abs();

        // 自適應輸出增益（未開啟則為 1.0）
        let adapt = if adaptive_cfg.enabled {
            map_gain(
                abs_e,
                adaptive_cfg.e_small,
                adaptive_cfg.e_large,
                adaptive_cfg.gain_min,
                adaptive_cfg.gain_max,
            )
        } else {
            1.0
        };

        // 實際下給馬達的命令（已套用自適應）並夾限
        let l_cmd = (l_raw * adapt).clamp(-1.0, 1.0);
        let r_cmd = (r_raw * adapt).clamp(-1.0, 1.0);

        // 驅動馬達（mock）
        let dist_dbg = dist_val.unwrap_or(f32::NAN);
        if let Err(e) = motor.set_wheel_speeds(l_cmd, r_cmd) {
            eprintln!("motor error: {e}");
        }
        if !args.quiet {
            println!(
                "t={t_sec:5.2}s dt={dt_s:.3}s d={dist_dbg:.2} v_des={desired_v:.2} \
                -> (L={l_cmd:.2}, R={r_cmd:.2}) state={st:?} meas=({:.2},{:.2}) e={:.3} gain={:.2}",
                v_meas_l, v_meas_r, ctrl_err, adapt
            );
        }

        // 記錄：把 err / adapt_gain 一併寫入
        let state_str = format!("{:?}", st);
        let ml = if args.bench { v_meas_l } else { f32::NAN };
        let mr = if args.bench { v_meas_r } else { f32::NAN };
        if let Err(io_err) = writeln!(
            writer,
            "{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},{},{:.3},{:.3},{:.4},{:.3}",
            t_sec, dt_s, desired_v, l_cmd, r_cmd, dist_dbg, state_str, ml, mr, ctrl_err, adapt
        ) {
            eprintln!("failed to write telemetry: {io_err}");
        }

        std::thread::sleep(dt);
    }

    if let Err(err) = writer.flush() {
        eprintln!("failed to flush telemetry csv: {err}");
    }

    eprintln!("Telemetry saved to {}", csv_path.display());
}
