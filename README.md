# robo-sim2real

[![CI](https://github.com/hoyichungg/robo-sim2real/actions/workflows/ci.yml/badge.svg)](https://github.com/hoyichungg/robo-sim2real/actions)

Unified control, simulation, and runtime scaffolding for a differential-drive robot. The workspace shares one control core across a Bevy-based 2D simulator and a Raspberry Pi runtime (with mock/bench drivers on desktop). The focus is a minimal loop with PID + fail-safe, deterministic telemetry, and tooling that stays in sync between sim and hardware.

---

## ✨ Highlights

- **Shared control core** (`r2_core`) consumed by both simulator and platform binaries.
- **Config layering** – TOML configs merge with CLI overrides; velocity profiles parse up front into `core_profile::VProfile` so the runtime never re-parses strings.
- **Bench plant** – First-order motor model lets the platform runner emulate dynamics (`--bench`, `--bench-tau`, `--bench-gain`) and records the simulated measurements back into telemetry.
- **Safety first** – Fail-safe clamps speed when distance drops below the configured threshold or sensors fail.
- **Telemetry everywhere** – Simulator and platform emit the same CSV schema (`t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain`), enabling shared analysis scripts.
- **Controllable logging** – Mock drivers log through `tracing::debug!`, keeping high-rate runs quiet unless you opt-in.
- **CI-ready** – `cargo test` exercises the control core and simulator configuration paths; GitHub Actions build + test the workspace.

---

## 🗂 Workspace Layout

```
.
├─ r2_core/          # PID, adaptive gain, fail-safe, profiles, and config merge logic
├─ drivers/          # Mock drivers (default) and optional Raspberry Pi implementations
├─ platform_rpi/     # CLI runner for bench mode or real hardware
├─ sim2d/            # Bevy simulation with CLI/config loader and telemetry writer
├─ configs/          # Example TOML configs for simulator runs
├─ docs/             # Architecture, API, tuning guides
├─ scripts/          # Telemetry plotting and analysis utilities
└─ tests/            # Cross-cutting integration tests (e.g. safety fail-safe)
```

---

## 🚀 Getting Started

### Prerequisites

- Rust 1.74+ (latest stable recommended), installed via `rustup`.
- For telemetry plotting: Python 3.10+ (optional) with pandas/matplotlib if you want to use the helper scripts.

### Run the 2D simulator

```bash
cargo run -p sim2d
```

The default run spawns a simple map with three obstacles and executes a constant-velocity profile. Use CLI flags to experiment:

```bash
cargo run -p sim2d -- \
  --hz 120 \
  --v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
  --tau 0.6 --plant-gain 0.9 \
  --adaptive --e-small 0.02 --e-large 0.18 --gain-min 0.6 --gain-max 1.4 \
  --obstacle 300,0 --obstacle 420,-50
```

Velocity profiles may be `const`, `step`, or `sin`; invalid values are rejected during CLI/config parsing.

### Use a config file

```bash
cp configs/sim2d.example.toml run/my_sim.toml
cargo run -p sim2d -- --config run/my_sim.toml

# CLI flags still override the file
cargo run -p sim2d -- --config run/my_sim.toml --hz 200 --plant-gain 1.0
```

Relative paths in the TOML file resolve against the file location, so you can keep per-run assets alongside the configuration snippet.

### Platform runner (desktop bench or Raspberry Pi)

```bash
# Desktop bench mode (first-order motor model + mock distance sensor)
cargo run -p platform_rpi -- \
  --bench --bench-tau 0.8 --bench-gain 0.6 \
  --hz 100 --seconds 8 \
  --v-profile step --step-at 3.0 \
  --adaptive --e-small 0.02 --e-large 0.20 --gain-min 0.6 --gain-max 1.2 \
  --csv run/telemetry.csv

# Suppress per-tick stdout by enabling tracing instead
RUST_LOG=drivers::mock_motor=debug cargo run -p platform_rpi -- --bench --quiet
```

To deploy on a Raspberry Pi, build with the hardware feature enabled (either directly on the Pi, or via your cross toolchain):

```bash
# Native build on the Pi
cargo build -p platform_rpi --release --features rpi

# Example cross (adjust target/toolchain as needed)
cargo build -p platform_rpi --release --features rpi \
  --target aarch64-unknown-linux-gnu
```

When the `rpi` feature is active, the driver crate switches from the mock implementations to the `rppal`-backed motor and distance sensor.

---

## ⚙️ Configuration Layering

- `sim2d` and `platform_rpi` both expose a CLI that maps cleanly onto `ControlOverrides`.
- TOML files (`configs/*.toml`) deserialize into `FileOverrides`. Applying a file, then the CLI, mirrors the merge order documented in `docs/API.md`.
- Velocity profiles are parsed exactly once when building `SimSettings`/`RuntimeCfg` and exposed as `Option<core_profile::VProfile>`; errors surface immediately instead of mid-run.
- Unit tests in `sim2d/src/config.rs` cover all merge paths (file-only, CLI-only, and CLI-over-file precedence) to keep future additions honest.

---

## 📊 Telemetry & Analysis

- Both binaries emit identical CSV headers: `t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain`.
- The simulator now writes the filtered plant velocity into `meas_left`/`meas_right`, allowing scripts to compare command vs. measured response without NaNs.
- Helper scripts in `scripts/`:
  - `plot_telemetry.py` – quick visualization of a single run.
  - `analyze_run.py` / `analyze_compare.py` – numeric summaries and overlays.
  - `run_pid_sweep.py` – launch multiple bench runs and collate the results.

---

## 🔊 Logging

Mock drivers avoid spamming stdout by logging through `tracing::debug!`. Opt-in per-module logging with:

```bash
RUST_LOG=drivers::mock_motor=debug cargo run -p platform_rpi -- --bench --quiet
```

Bring your own subscriber (e.g. `tracing-subscriber`) in binaries or integration tests if you need structured output.

---

## 🧪 Testing

```bash
cargo test
cargo test -p sim2d
```

The workspace exercises PID math, adaptive gain interpolation, safety fail-safe behaviour, and the new configuration merge paths. Running simulator tests also ensures Bevy schedules compile with current features.

---

## 📚 Documentation

- `docs/ARCHITECTURE.md` – module overview and data flow.
- `docs/API.md` – public structs, CLI/config reference, telemetry schema.
- `docs/TUNING.md` – PID, fail-safe, and plant tuning walkthroughs.

---

## ⚖️ License

Released under the [MIT License](./LICENSE).
