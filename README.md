<!--
File: README.md
Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
Version: v0.1.6-rtt-adc-live-value
Purpose: Project notes and bring-up roadmap for Rust firmware on the B-G431B-ESC1 / STM32G431CB ESC board
Created: 2026-06-07
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
  - Not the MCU being programmed by this project

ESC / target section
  - Contains the STM32G431CB target MCU
  - Contains motor-control hardware, gate drivers, MOSFETs, sensors, and board I/O
  - This is the MCU running the Rust firmware

We are writing firmware for:

STM32G431CB
Cortex-M4F
128 KiB Flash
32 KiB SRAM
Rust target: thumbv7em-none-eabihf
Current Confirmed Milestone

Current firmware version:

v0.1.6-rtt-adc-live-value

Confirmed working:

- ST-LINK detection
- probe-rs flashing
- cargo-embed RTT terminal output
- Rust no_std firmware boot
- linker/memory.x setup
- PC6 STATUS LED output
- PC10 user button input
- PB12 potentiometer ADC input
- live ADC value monitor over RTT/SWD

Confirmed Board Pin Notes

From schematic inspection and testing:

PC6   = STATUS LED output
PC10  = user button input, active-low
PB12  = potentiometer input
PB12  = ADC1_IN11
PB14 = temperature feedback / ADC1_IN5
Confirmed raw value increases when board is warmed by hand and falls back toward ambient.

Observed potentiometer behavior:

pot far right  = low ADC value, near 0
pot far left   = high ADC value, up to 4095

Current firmware behavior:

button released:
  potentiometer controls STATUS LED blink speed

button held:
  fixed fast blink override

RTT terminal:
  prints live_raw, live_pct, delay_cycles, button state, and timeout state
Useful Commands

Detect ST-LINK:

probe-rs list

Detect STM32 with stlink tools:

st-info --probe

Build release firmware:

cargo build --release

Flash and run with probe-rs:

cargo run --release

Flash and monitor RTT:

cargo embed --release

Attach to already-running firmware without reflashing:

probe-rs attach --chip STM32G431CB target/thumbv7em-none-eabihf/release/b-g431b-esc1-rust

Exit RTT/probe session:

Ctrl-C
Current Project Files
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
Safety Rules

Early bring-up must avoid motor-control hardware until the board is better understood.

Do not connect:

- motor battery
- motor
- propeller

until PWM, gate-driver enable, fault inputs, current sensing, and protection behavior are understood.

Current safe-tested features only touch:

PC6  STATUS LED
PC10 button
PB12 potentiometer ADC
RTT over ST-LINK/SWD
Bring-up Roadmap
Stage 1 — Completed

Basic Rust firmware pipeline:

- backup original flash
- configure Rust target
- create no_std project
- memory.x linker setup
- flash with probe-rs
- blink PC6 STATUS LED
Stage 2 — Completed

Basic GPIO input:

- configure PC10 button input
- confirm active-low behavior
- use button to change LED blink pattern
Stage 3 — Completed

Basic ADC input:

- configure PB12 as analog input
- enable ADC1
- read ADC1_IN11
- print live ADC values over RTT
- use potentiometer to control blink speed
Stage 4 — Next Suggested Step

Map and read more board signals.

Candidate signals:

- DC bus voltage sense ADC
- current sense ADC inputs
- temperature feedback
- gate-driver fault pins
- overcurrent / protection pins

Goal:

RTT output shows live board telemetry before any motor switching.

Example target output:

pot_raw=1234  vbus_raw=2870  temp_raw=1900  fault=0  button=0
Stage 5 — Timer Bring-up, No Power Stage

Configure motor PWM timer without enabling the gate drivers.

Likely area of work:

TIM1 complementary PWM
dead-time setup
PWM frequency setup
safe default duty cycle
all outputs disabled at startup

Safety state:

gate drivers disabled
no motor battery
no motor attached
Stage 6 — Gate Driver / Fault Bring-up

Only after schematic pin mapping:

- identify enable/shutdown pins
- identify fault pins
- enable driver carefully
- read fault status
- verify safe startup and shutdown behavior

Safety state:

current-limited bench supply
no propeller
prefer no motor at first
Stage 7 — First Motor Experiment

First actual motor-control goal:

open-loop 6-step commutation
very low duty cycle
small BLDC motor
current-limited supply
no propeller

Initial motor milestone:

motor twitches or slowly spins under controlled open-loop sequence
Current Notes

RTT output is carried through the ST-LINK debug connection, not UART. The large LED near the USB connector may show activity while probe-rs or cargo-embed is attached and reading RTT data.

The ST-LINK section remains intact after flashing this project because the firmware is written to the STM32G431CB target MCU on the ESC section.

Git Milestones

Current baseline commit:

v0.1.6 STM32G431 Rust bringup with GPIO ADC and RTT

Recommended future tags:

v0.1.6-rtt-adc-live-value
v0.1.7-vbus-adc-monitor
v0.1.8-fault-pin-monitor
v0.2.0-tim1-pwm-disabled-output-test
v0.3.0-gate-driver-safe-enable-test
v0.4.0-open-loop-6step-first-spin
<!-- Footer File: README.md Version: v0.1.6-rtt-adc-live-value Created: 2026-06-07 Generated timestamp: 2026-06-07 -->
