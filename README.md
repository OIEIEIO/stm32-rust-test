<!--
File: README.md
Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
Version: v0.1.8-pa0-vbus-raw-monitor
Purpose: Project notes and bring-up roadmap for Rust firmware on the B-G431B-ESC1 / STM32G431CB ESC board
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->

# STM32 Rust Test — B-G431B-ESC1 ESC Bring-up

Rust firmware bring-up project for the **ST B-G431B-ESC1 Discovery ESC board**.

The goal is to learn the board step by step, starting with safe GPIO and ADC tests, then progressing toward PWM, gate-driver bring-up, and eventually spinning a small BLDC motor.

## Board Understanding

This development board has two major sections:

```text
USB / ST-LINK section
  - Provides USB connection to the PC
  - Acts as debugger / programmer
  - Supports probe-rs, cargo-embed, SWD, and RTT output
  - Has its own USB/activity LEDs
  - Not the MCU being programmed by this project

ESC / target section
  - Contains the STM32G431CB target MCU
  - Contains motor-control hardware, gate drivers, MOSFETs, sensors, and board I/O
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
v0.1.8-pa0-vbus-raw-monitor
```

Confirmed working:

```text
- ST-LINK detection
- probe-rs flashing
- cargo-embed RTT terminal output
- probe-rs attach monitoring without reflashing
- Rust no_std firmware boot
- linker/memory.x setup
- PC6 STATUS LED output
- PC10 user button input, active-low
- PB12 potentiometer ADC input
- PB14 temperature feedback ADC input
- PA0 VBUS feedback ADC input
- live board telemetry over RTT/SWD
```

## Confirmed Board Pin Notes

From schematic inspection and testing:

```text
PC6   = STATUS LED output
PC10  = user button input, active-low
PB12  = potentiometer input / ADC1_IN11
PB14  = temperature feedback / ADC1_IN5
PA0   = VBUS feedback / ADC1_IN1
```

Observed potentiometer behavior:

```text
pot far right  = low ADC value, near 0
pot far left   = high ADC value, up to 4095
```

Observed temperature-feedback behavior:

```text
PB14 temp_raw increases when the ESC board is warmed by hand.
PB14 temp_raw falls back toward ambient after release.
Values are raw ADC counts only, not Celsius.
```

Observed VBUS-feedback behavior:

```text
USB-only / USB-backfed ESC input:
  measured ESC input voltage: about 4.73 V
  vbus_raw baseline: about 577

External bench supply test:
  supply: current-limited to 100 mA
  raised from about 5.67 V up to about 10 V
  vbus_raw rose to about 1214 at about 10 V
  vbus_raw fell again when supply voltage was reduced
```

Bench-derived rough calibration:

```text
vbus_raw counts per volt ≈ 121 to 122 counts/V
rough estimate: VBUS volts ≈ vbus_raw / 121.5
```

This calibration is **bench-derived only**. The final voltage scaling should be confirmed later from the actual VBUS resistor-divider schematic values.

## Current Firmware Behavior

Button released:

```text
potentiometer controls STATUS LED blink speed
```

Button held:

```text
fixed fast blink override
```

RTT terminal output includes:

```text
button=<0/1>
pot_raw=<0..4095>
pot_pct=<0..100>
temp_raw=<0..4095>
temp_delta=<raw-startup>
vbus_raw=<0..4095>
vbus_delta=<raw-startup>
delay_cycles=<value>
timeout=<0/1>
mode=<pot_control/button_fast>
```

Current note: the telemetry print rate is tied to the same main loop as the LED blink. When the potentiometer increases the blink speed, the RTT log output also scrolls faster. This is expected with the current simple loop structure.

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

Early bring-up must avoid motor-control hardware until the board is better understood.

Do not connect:

```text
- motor
- propeller
```

until PWM, gate-driver enable, fault inputs, current sensing, and protection behavior are understood.

When applying ESC input power from a bench supply:

```text
- confirm polarity before connection
- start with low current limit
- raise voltage gradually
- monitor vbus_raw over RTT
- stop if current jumps, board heats unexpectedly, ST-LINK disconnects, or anything smells wrong
```

Current safe-tested features only touch:

```text
PC6   STATUS LED
PC10  button
PB12  potentiometer ADC
PB14  temperature feedback ADC
PA0   VBUS feedback ADC
RTT over ST-LINK/SWD
```

Current firmware does **not** configure TIM1 PWM outputs and does **not** enable gate-driver switching.

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

Basic potentiometer ADC input:

```text
- configure PB12 as analog input
- enable ADC1
- read ADC1_IN11
- print live ADC values over RTT
- use potentiometer to control blink speed
```

### Stage 4 — Completed

Raw board telemetry monitor:

```text
- read PB12 potentiometer ADC1_IN11
- read PB14 temperature feedback ADC1_IN5
- read PA0 VBUS feedback ADC1_IN1
- print live board telemetry over RTT
```

Confirmed observations:

```text
- pot_raw changes with the onboard potentiometer
- temp_raw rises when board is warmed by hand
- vbus_raw rises/falls with real ESC input voltage from a current-limited bench supply
```

### Stage 5 — Next Suggested Step

Map and read more board protection/current feedback signals before PWM.

Candidate schematic signals:

```text
- current feedback / op-amp related pins
- gate-driver fault pins
- overcurrent / protection pins
- enable / shutdown pins
```

Important distinction for current feedback:

```text
Some schematic labels are op-amp input pins.
Before coding, identify which nets are actual ADC-readable outputs versus op-amp inputs.
```

Goal:

```text
RTT output shows useful board telemetry and protection status before any motor switching.
```

Example target output:

```text
pot_raw=1234  temp_raw=1900  vbus_raw=2870  fault=0  button=0
```

### Stage 6 — Optional Firmware Cleanup

Separate the main-loop timing:

```text
LED blink rate = controlled by potentiometer
RTT log rate   = fixed, for example 2 to 5 logs per second
ADC read rate  = fixed or faster than logging
```

This is not required for board bring-up, but it will make telemetry easier to read as more signals are added.

### Stage 7 — Timer Bring-up, No Power Stage

Configure motor PWM timer without enabling the gate drivers.

Likely area of work:

```text
TIM1 complementary PWM
dead-time setup
PWM frequency setup
safe default duty cycle
all outputs disabled at startup
```

Safety state:

```text
gate drivers disabled
no motor attached
no propeller
```

### Stage 8 — Gate Driver / Fault Bring-up

Only after schematic pin mapping:

```text
- identify enable/shutdown pins
- identify fault pins
- enable driver carefully
- read fault status
- verify safe startup and shutdown behavior
```

Safety state:

```text
current-limited bench supply
no propeller
prefer no motor at first
```

### Stage 9 — First Motor Experiment

First actual motor-control goal:

```text
open-loop 6-step commutation
very low duty cycle
small BLDC motor
current-limited supply
no propeller
```

Initial motor milestone:

```text
motor twitches or slowly spins under controlled open-loop sequence
```

## Current Notes

RTT output is carried through the ST-LINK debug connection, not UART. The large LED near the USB connector may show activity while probe-rs or cargo-embed is attached and reading RTT data.

The ST-LINK section remains intact after flashing this project because the firmware is written to the STM32G431CB target MCU on the ESC section.

The VBUS feedback input is confirmed live, but the final volts conversion is not yet schematic-confirmed.

## Git Milestones

Current local baseline to commit:

```text
v0.1.8 PA0 VBUS raw monitor confirmed with current-limited bench supply
```

Recommended future tags:

```text
v0.1.6-rtt-adc-live-value
v0.1.7-pb14-temperature-feedback-raw
v0.1.8-pa0-vbus-raw-monitor
v0.1.9-current-feedback-raw-monitor
v0.2.0-tim1-pwm-disabled-output-test
v0.3.0-gate-driver-safe-enable-test
v0.4.0-open-loop-6step-first-spin
```

<!--
Footer
File: README.md
Version: v0.1.8-pa0-vbus-raw-monitor
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->
