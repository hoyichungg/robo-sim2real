use bevy::prelude::*;

#[derive(Component)]
pub struct Car;

#[derive(Component)]
pub struct Obstacle;

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Velocity {
    pub cmd: f32,   // 控制器輸出的線速度命令（m/s）
    pub meas: f32,  // 模擬後的量測速度（m/s）
    pub omega: f32, // 角速度（rad/s）先保留彈性
}

/// 車頭朝向（單位向量），2D
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Heading(pub Vec2);

impl Heading {
    pub fn forward(&self) -> Vec2 {
        self.0.normalize_or_zero()
    }
}
