# Raspberry Pi 實機驅動（GPIO / PWM / 超音波）

本文件說明如何在 Raspberry Pi 上啟用實機驅動，使用硬體 PWM 控制馬達，與 HC‑SR04 超音波感測距離。預設專案仍使用 mock/stub，若要切換實機請按照以下步驟。

## 功能概覽
- 馬達：`drivers::rpi::RpiMotor`（兩路硬體 PWM）。
  - 需對應支援 PWM 的 Channel（如 PWM0/PWM1），請依 Pi 型號與接腳查表（GPIO12/13/18/19 常見）。
  - 以 `max_mps` 將 m/s 線性映射到 duty（|v|>=max_mps → duty=100%）。方向控制需搭配 H 橋或方向腳，本文範例先忽略方向僅做單向。
- 距離：`drivers::rpi::RpiDistance`（HC‑SR04）。
  - 以 `trig` 送 10µs 脈衝，量測 `echo` 高電位寬度換算距離。

## 啟用方式

1) 在 RPi 上編譯與執行，並啟用 feature：

```bash
# 啟用 drivers 的 rpi feature
cargo run -p platform_rpi --features rpi -- --help
```

2) 在程式中切換為實機驅動（示意）：

```rust
// platform_rpi/src/main.rs (範例片段)
#[cfg(feature = "rpi")]
use drivers::rpi::{RpiDistance, RpiMotor};
#[cfg(feature = "rpi")]
use rppal::pwm::Channel;

fn main() {
    // ... 解析 CLI 省略 ...

    #[cfg(feature = "rpi")]
    let mut motor = RpiMotor::new(Channel::Pwm0, Channel::Pwm1, 1000.0, 1.0)
        .expect("init pwm motor");
    #[cfg(not(feature = "rpi"))]
    let mut motor = drivers::rpi::RpiMotorStub; // 預設 stub

    #[cfg(feature = "rpi")]
    let mut sensor = RpiDistance::new(23, 24).expect("init hcsr04 (BCM GPIO23 trig, 24 echo)");
    #[cfg(not(feature = "rpi"))]
    let mut sensor = drivers::rpi::RpiDistanceStub; // 預設 stub

    // ... 後續控制迴圈不變 ...
}
```

> 提示：若你不想在 `main.rs` 寫 `cfg`，也可在 `drivers` 提供一個工廠函式依 feature 回傳對應型別。

## 接線說明（範例）
- PWM：依你的馬達驅動板支援的 PWM 腳接到對應的 EN/IN 腳位。
  - ex. 左馬達用 `Channel::Pwm0`（GPIO12/18 之一），右馬達用 `Channel::Pwm1`（GPIO13/19 之一）。
  - 注意硬體 PWM 與實際 GPIO 的對應，因 Pi 型號與引腳功能不同，請參考 rppal 與官方 pinout。
- HC‑SR04：
  - `VCC` → 5V、`GND` → GND
  - `TRIG` → BCM GPIO23（可自行調整）
  - `ECHO` → BCM GPIO24（可自行調整；建議分壓到 3.3V）

## 參數建議
- PWM 頻率 `freq_hz`：可從 1000 Hz 起測，依你的電機驅動器建議調整。
- `max_mps`：設定你的車在滿 PWM 時的最大線速（m/s），讓 PID 輸出與實際馬達具合理對應。

## 限制與注意
- 目前 `RpiMotor` 僅以 duty 代表「推力大小」，未包含方向切換。實務上請搭配 H 橋或雙 PWM 實作前進/後退。
- HC‑SR04 需要 5V 供電，`ECHO` 腳需做分壓或電平轉換至 3.3V 再接入 GPIO。
- 本驅動為最小可行範例，實機可能需要：
  - 方向控制腳邏輯、死區/線性化、飽和/電流保護
  - 距離去噪與多次量測中值濾波
  - 更換成 I2C ToF（如 VL53L0X）時需加入對應驅動

## 疑難排解
- PWM 不動作：確認使用的是對應 PWM 的 GPIO（如 12/13/18/19），且你的 HAT/擴展板支援硬體 PWM 輸入。
- HC‑SR04 timeout：檢查接線與分壓、室內環境與障礙物距離是否在量測範圍內；可增加 timeout 或加上重試機制。

---

若你希望我幫你在 `platform_rpi` 加上 feature 封裝（例如 `features = ["rpi"]` 自動連動 drivers/rpi），或提供工廠方法來簡化初始化，請告訴我。

