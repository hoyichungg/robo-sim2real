use crate::resources::TelemetryWriter; // 去掉 SimClock，避免 unused import 警告
use bevy::prelude::*;

/// 依你自己的 Writer 設計：這裡先保留「每個 fixed step flush 一次」的骨架
pub fn flush_telemetry(_writer: ResMut<TelemetryWriter>) {
    // 如果 TelemetryWriter 有 buffer，就在這裡寫檔並清空
    // 目前先留空，避免因未知 API 導致編譯失敗
}
