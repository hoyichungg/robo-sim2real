use r2_core::control::safety::{FailSafe, SafetyState};

#[test]
fn enters_emergency_brake_on_close_distance_or_error() {
    let mut fs = FailSafe::new(0.25, 0.05);
    assert_eq!(fs.state(), SafetyState::Run);

    // 低於門檻 → 立即急停
    let st = fs.update(Ok(0.20));
    assert_eq!(st, SafetyState::EmergencyBrake);
    assert_eq!(fs.clamp_speed(0.6), 0.0);

    // 感測錯誤也要急停
    let mut fs2 = FailSafe::new(0.25, 0.05);
    let st2 = fs2.update(Err(()));
    assert_eq!(st2, SafetyState::EmergencyBrake);
}

#[test]
fn err_or_nan_enters_emergency() {
    let mut fs = FailSafe::new(0.25, 0.05);
    assert_eq!(fs.update(Err(())), SafetyState::EmergencyBrake);

    let mut fs2 = FailSafe::new(0.25, 0.05);
    assert_eq!(fs2.update(Ok(f32::NAN)), SafetyState::EmergencyBrake);
}

#[test]
fn threshold_triggers_and_safe_stop_holds_zero() {
    let mut fs = FailSafe::new(0.25, 0.05);
    assert_eq!(fs.update(Ok(0.20)), SafetyState::EmergencyBrake);
    assert_eq!(fs.update(Ok(0.30)), SafetyState::SafeStop);
    assert_eq!(fs.clamp_speed(0.6), 0.0);
}

#[test]
fn reset_requires_hysteresis_margin() {
    let mut fs = FailSafe::new(0.25, 0.05);
    fs.update(Ok(0.1)); // emergency
    fs.update(Ok(0.4)); // safe stop

    // 還沒超過 threshold + hysteresis (=0.30)，不應解除
    fs.reset(Some(0.29));
    assert_ne!(fs.state(), SafetyState::Run);

    // 超過 0.30 才能解除
    fs.reset(Some(0.31));
    assert_eq!(fs.state(), SafetyState::Run);
}

#[test]
fn negative_distance_treated_as_danger() {
    let mut fs = FailSafe::new(0.25, 0.05);
    assert_eq!(fs.update(Ok(-1.0)), SafetyState::EmergencyBrake);
}
