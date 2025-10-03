use bevy::prelude::*;
use clap::Parser;

mod components;
mod config;
mod control;
mod logging;
mod physics;
mod resources;
mod sensing;

use components::*;
use config::{Cli, RuntimeCfg};
use control::control_step;
use physics::integrate_kinematics;
use resources::{DistanceSense, SimClock, TelemetryWriter};
use sensing::sense_distance;

fn main() {
    let cli = Cli::parse();

    App::new()
        // 視窗 / 渲染
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Sim2D Robot".into(),
                    resolution: (1000.0, 700.0).into(),
                    ..default()
                }),
                ..default()
            }), // 建議把 vsync 也留預設
        )
        // 設定 FixedUpdate 頻率
        .insert_resource(Time::<Fixed>::from_hz(cli.hz as f64))
        .insert_resource(SimClock {
            t: 0.0,
            dt: 1.0 / cli.hz,
        })
        .insert_resource(DistanceSense(f32::INFINITY))
        .insert_resource(TelemetryWriter::new(cli.csv.clone()))
        .insert_resource(cli.to_runtime())
        // 場景
        .add_systems(Startup, (spawn_camera, spawn_scene))
        // 感測 → 控制 → 物理 → 紀錄（固定步進）
        .add_systems(
            FixedUpdate,
            (
                sense_distance,
                control_step,
                integrate_kinematics,
                logging::flush_telemetry,
            )
                .chain(),
        )
        // 時鐘遞增
        .add_systems(FixedUpdate, tick_clock)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_scene(mut commands: Commands, cfg: Res<RuntimeCfg>) {
    // 車
    commands.spawn((
        Car,
        Heading(Vec2::X), // 朝 +X
        Velocity::default(),
        SpriteBundle {
            sprite: Sprite {
                // 0.14 建議用 srgb
                color: Color::srgb(0.2, 0.6, 0.9),
                custom_size: Some(Vec2::new(60.0, 40.0)),
                ..default()
            },
            transform: Transform::from_xyz(-300.0, 0.0, 0.0),
            ..default()
        },
    ));

    // 多障礙（CLI 提供座標；若未提供則給預設三個）
    let mut list: Vec<Vec2> = if cfg.obstacles.is_empty() {
        vec![
            Vec2::new(300.0, 0.0),
            Vec2::new(350.0, 120.0),
            Vec2::new(420.0, -100.0),
        ]
    } else {
        cfg.obstacles.clone()
    };
    let colors = [
        Color::srgb(0.6, 0.6, 0.6),
        Color::srgb(0.7, 0.5, 0.5),
        Color::srgb(0.5, 0.7, 0.5),
        Color::srgb(0.5, 0.5, 0.8),
    ];
    for (i, p) in list.drain(..).enumerate() {
        let col = colors[i % colors.len()];
        commands.spawn((
            Obstacle,
            SpriteBundle {
                sprite: Sprite {
                    color: col,
                    custom_size: Some(Vec2::new(80.0, 80.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(p.x, p.y, 0.0)),
                ..default()
            },
        ));
    }
}

fn tick_clock(time: Res<Time<Fixed>>, mut clk: ResMut<SimClock>) {
    clk.dt = time.delta_seconds();
    clk.t += clk.dt;
}
