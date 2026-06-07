<!--
File: README.md
Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
Version: v0.2.4-tim1-moe-on-ccer-off
Purpose: Project notes and bring-up roadmap for Rust firmware on the B-G431B-ESC1 / STM32G431CB ESC board
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->

# STM32 Rust Test — B-G431B-ESC1 ESC Bring-up

Rust firmware bring-up project for the **ST B-G431B-ESC1 Discovery ESC board**.

The goal is to learn the board step by step, starting with safe GPIO and ADC tests, then progressing toward TIM1 motor-control setup, gate-driver bring-up, and eventually spinning a small BLDC motor.

## Board Understanding

This development board has two major sections:

```text
USB / ST-LINK section
  - Provides USB connection to the PC
  - Acts as debugger / programmer
  - Supports probe-rs, cargo-embed, SWD, and RTT output
  - Not the MCU being programmed by this project

ESC / target section
  - Contains the STM32G431CB target MCU
  - Contains motor-control hardware, L6387 gate drivers, MOSFETs, sensors, shunts, and board I/O
  - This is the MCU running the Rust firmware
```

We are writing firmware for:

```text
STM32G431CB
Cortex-M4F
128 KiB Flash
32 KiB SRAM
Rust target: thumbv7em-none-eabihf
```

## Current Confirmed Milestone

Current firmware version:

```text
v0.2.4-tim1-moe-on-ccer-off
```

Confirmed working:

```text
- ST-LINK detection
- probe-rs flashing
- cargo-embed RTT terminal output
- Rust no_std firmware boot
- linker/memory.x setup
- PC6 STATUS LED output
- PC10 user button input
- PB12 potentiometer ADC input
- PB14 raw temperature feedback ADC input
- PA0 raw VBUS feedback ADC input
- OP1/OP2/OP3 raw current-feedback monitor
- TIM1 internal counter setup
- TIM1 drive pins moved to alternate-function mode
- TIM1 CCER enabled milestone passed in v0.2.3-fix1 with MOE off
- TIM1 MOE can be enabled while CCER remains off
- TIM1 telemetry/readback continues working over RTT
```

## Confirmed Board Pin Notes

From schematic inspection and testing:

```text
PC6   = STATUS LED output
PC10  = user button input, active-low
PB12  = potentiometer input / ADC1_IN11
PB14  = temperature feedback / ADC1_IN5
PA0   = VBUS feedback / ADC1_IN1
PA2   = OP1_OUT raw monitor / ADC1_IN3
PA6   = OP2_OUT raw monitor / ADC2_IN3
PB1   = OP3_OUT raw monitor / ADC1_IN12
```

Observed potentiometer behavior:

```text
pot far right  = low ADC value, near 0
pot far left   = high ADC value, up to 4095
```

Observed temperature feedback behavior:

```text
PB14 temp_raw increases when the ESC board is warmed by hand
PB14 temp_raw falls back toward ambient after release
```

Observed VBUS behavior:

```text
USB-only / USB backfeed: approximately 4.73 V seen at ESC input, vbus_raw around 577
Bench supply around 10 V: vbus_raw around 1214
Rough bench-derived scale: about 121 ADC counts per volt
Treat this as approximate until schematic divider values are confirmed
```

Observed zero-current raw op-amp offsets, no motor/load:

```text
OP1 raw around 1590-1600 range
OP2 raw around 1590-2000 range depending on run/version/readback state
OP3 raw around 1420-1520 range depending on run/version/readback state
These are raw zero-current baselines, not calibrated current values
```

## TIM1 / Gate Driver Mapping

From schematic inspection:

```text
U phase:
  PA8   = UH = TIM1_CH1  -> L6387 HIN
  PC13  = UL = TIM1_CH1N -> L6387 LIN
  output = OUT1

V phase:
  PA9   = VH = TIM1_CH2  -> L6387 HIN
  PA12  = VL = TIM1_CH2N -> L6387 LIN
  output = OUT2

W phase:
  PA10  = WH = TIM1_CH3  -> L6387 HIN
  PB15  = WL = TIM1_CH3N -> L6387 LIN
  output = OUT3
```

The L6387 inputs appear driven directly from STM32G431 TIM1 complementary outputs. No separate gate-driver enable/fault pin has been identified in the reviewed schematic crop.

## Current v0.2.4 Behavior

This version keeps the power stage in a non-switching test state:

```text
Drive pins are TIM1 alternate function.
TIM1 counter is running.
TIM1 MOE is ON.
TIM1 CCER is OFF.
TIM1 CCR1/CCR2/CCR3 remain 0.
No intentional gate switching in this version.
```

Expected confirmed fields from RTT:

```text
safety_ok=1
af_ok=1
tim1_counting=1
tim1_ccer=0
tim1_moe=1
tim1_ossi=1
tim1_ossr=1
tim1_ccr1=0
tim1_ccr2=0
tim1_ccr3=0
UH/VH/WH/VL/WL/UL pin readbacks = 0
```

This proves that MOE can be enabled while capture/compare output enables remain disabled. It does **not** yet intentionally switch MOSFET gates.

## Useful Commands

Detect ST-LINK:

```bash
probe-rs list
```

Detect STM32 with stlink tools:

```bash
st-info --probe
```

Build release firmware:

```bash
cargo build --release
```

Flash and run with probe-rs:

```bash
cargo run --release
```

Flash and monitor RTT:

```bash
cargo embed --release
```

Attach to already-running firmware without reflashing:

```bash
probe-rs attach --chip STM32G431CB target/thumbv7em-none-eabihf/release/b-g431b-esc1-rust
```

Exit RTT/probe session:

```text
Ctrl-C
```

## Current Project Files

```text
b-g431b-esc1-rust/
├── .cargo/
│   └── config.toml
├── Cargo.toml
├── Cargo.lock
├── Embed.toml
├── build.rs
├── memory.x
├── README.md
└── src/
    └── main.rs
```

## Safety Rules

Early bring-up must avoid uncontrolled motor-control hardware behavior.

Do not connect:

```text
- propeller
- phase-to-phase resistor load during non-switching firmware
- motor unless the firmware version explicitly calls for first-spin testing
```

Current v0.2.4 safe-tested state:

```text
USB-only test passed
bench supply may remain off for this milestone
TIM1 MOE on, CCER off
no intentional gate switching
```

For future powered ESC tests:

```text
Use current-limited bench supply
Start at low voltage/current
No propeller
Keep hands clear of motor shaft and leads
Stop immediately if current jumps, board heats, ST-LINK disconnects, or unexpected behavior occurs
```

## Bring-up Roadmap

### Stage 1 — Completed

Basic Rust firmware pipeline:

```text
- backup original flash
- configure Rust target
- create no_std project
- memory.x linker setup
- flash with probe-rs
- blink PC6 STATUS LED
```

### Stage 2 — Completed

Basic GPIO input:

```text
- configure PC10 button input
- confirm active-low behavior
- use button to change LED blink pattern
```

### Stage 3 — Completed

Basic ADC input:

```text
- configure PB12 as analog input
- enable ADC1
- read ADC1_IN11
- print live ADC values over RTT
- use potentiometer to control blink speed
```

### Stage 4 — Completed

Temperature feedback raw monitor:

```text
- configure PB14 as analog input
- read ADC1_IN5
- print temp_raw and temp_delta
- confirm raw value increases when board is warmed by hand
```

### Stage 5 — Completed

VBUS raw monitor:

```text
- configure PA0 as analog input
- read ADC1_IN1
- confirm vbus_raw changes with ESC input voltage
- bench-derived rough scale around 121 counts/volt
```

### Stage 6 — Completed

Raw current-feedback op-amp output monitor:

```text
- PA2 OP1_OUT / ADC1_IN3
- PA6 OP2_OUT / ADC2_IN3
- PB1 OP3_OUT / ADC1_IN12
- confirm stable zero-current raw offsets
```

### Stage 7 — Completed

Drive pins safe-low GPIO state:

```text
- configure UH/UL/VH/VL/WH/WL pins as GPIO outputs
- force all six LOW
- confirm STM32 register/readback state
```

### Stage 8 — Completed

TIM1 internal counter, outputs disabled:

```text
- enable TIM1 clock
- configure PSC/ARR/CCR registers
- start internal counter
- keep CCER = 0
- keep BDTR.MOE = 0
- keep drive pins GPIO LOW
```

### Stage 9 — Completed

TIM1 alternate-function pins, MOE off:

```text
- move six drive pins to TIM1 AF
- verify AF mapping:
  PA8/PA9/PA10/PA12 = AF6
  PB15/PC13 = AF4
- keep CCER = 0
- keep BDTR.MOE = 0
```

### Stage 10 — Completed

TIM1 CCER enabled, MOE off:

```text
- TIM1 owns six drive pins
- enable CCER bits for CH1/1N/2/2N/3/3N
- keep BDTR.MOE = 0
- keep CCR1/2/3 = 0
- confirm safety_ok=1 in v0.2.3-fix1
```

### Stage 11 — Completed

TIM1 MOE on, CCER off:

```text
- TIM1 pins remain AF
- TIM1 counter running
- BDTR.MOE = 1
- CCER = 0
- CCR1/2/3 = 0
- no intentional gate switching
```

### Stage 12 — Next Suggested Step

TIM1 forced-inactive outputs with MOE and CCER enabled:

```text
v0.2.5-tim1-forced-inactive-ccer-on-moe-on

Goal:
- pins remain TIM1 AF
- channels configured forced inactive
- CCER enabled
- MOE enabled
- no PWM duty
- no motor yet
- verify no current jump under safe test conditions
```

After this, the next larger step would be a carefully designed first low-duty or forced-state hardware test with current-limited bench power, still no propeller.

## Git Milestones

Known milestones:

```text
v0.1.6-rtt-adc-live-value
v0.1.7-pb14-temperature-feedback-raw
v0.1.8-pa0-vbus-raw-monitor
v0.1.9-opamp-output-raw-monitor
v0.2.0-drive-pins-safe-low-readback
v0.2.1-tim1-internal-counter-moe-off
v0.2.2-tim1-af-pins-moe-off
v0.2.3-fix1-tim1-ccer-enabled-moe-off
v0.2.4-tim1-moe-on-ccer-off
```

<!--
Footer
File: README.md
Version: v0.2.4-tim1-moe-on-ccer-off
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->
