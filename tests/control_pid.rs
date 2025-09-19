use r2_core::control::pid::Pid;

// 簡單一階慣性系統
fn plant_step(y: f32, u: f32, dt: f32, tau: f32) -> f32 {
    y + dt * (u - y) / tau
}

#[test]
fn pid_converges_to_setpoint_on_first_order_plant() {
    let dt = 0.01;
    let tau = 0.5;
    let target = 1.0;
    let steps = (10.0 / dt) as usize;

    let mut pid = Pid::new(1.2, 1.0, 0.05)
        .with_output_limits(-10.0, 10.0)
        .with_integral_limits(-5.0, 5.0);

    let mut y = 0.0;
    for _ in 0..steps {
        let u = pid.step(target, y, dt);
        y = plant_step(y, u, dt, tau);
    }

    assert!(
        (target - y).abs() < 0.05,
        "final error too large: {err}, y={y}"
    );
}
