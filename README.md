<!--
File: README.md
Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
Version: v0.2.1-tim1-internal-counter-moe-off
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
  - Not the MCU being programmed by this project

ESC / target section
  - Contains the STM32G431CB target MCU
  - Contains motor-control hardware, L6387 half-bridge drivers, MOSFETs, sensors, and board I/O
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
v0.2.1-tim1-internal-counter-moe-off
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
- PB14 temperature feedback ADC input
- PA0 VBUS raw ADC input
- OP1/OP2/OP3 raw current-feedback output monitor
- six L6387 drive input pins forced LOW as GPIO outputs
- TIM1 internal counter configured and running
- TIM1 outputs disabled
- TIM1 BDTR.MOE held OFF
- drive pins remain GPIO LOW while TIM1 runs internally
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
PB14 temp_raw falls back toward ambient when released
```

Observed VBUS behavior:

```text
USB / ESC input observed around 4.73 V  -> vbus_raw around 577
Bench supply around 10 V                -> vbus_raw around 1214
```

Bench-derived rough scale:

```text
vbus_volts ≈ vbus_raw / 121.5
```

This VBUS scale is only a bench-derived estimate until the divider values are confirmed from the schematic.

Observed raw OPAMP/current-feedback output baselines with no motor current:

```text
op1_raw ≈ 1590–1615
op2_raw ≈ 1970–1999
op3_raw ≈ 1500–1525
```

These are treated as zero-current raw offsets at this stage.

## Gate Driver / Phase Drive Pins

From schematic inspection:

```text
U phase:
  PA8  = UH = TIM1_CH1  -> L6387 HIN
  PC13 = UL = TIM1_CH1N -> L6387 LIN
  phase output = OUT1

V phase:
  PA9  = VH = TIM1_CH2  -> L6387 HIN
  PA12 = VL = TIM1_CH2N -> L6387 LIN
  phase output = OUT2

W phase:
  PA10 = WH = TIM1_CH3  -> L6387 HIN
  PB15 = WL = TIM1_CH3N -> L6387 LIN
  phase output = OUT3
```

Current firmware state:

```text
PA8  UH = GPIO output LOW
PC13 UL = GPIO output LOW
PA9  VH = GPIO output LOW
PA12 VL = GPIO output LOW
PA10 WH = GPIO output LOW
PB15 WL = GPIO output LOW
```

`v0.2.0` confirmed GPIO readback:

```text
drive_safe=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

`v0.2.1` keeps those six pins as GPIO LOW while TIM1 runs internally only.

## TIM1 Internal Counter Test

`v0.2.1-tim1-internal-counter-moe-off` configures TIM1 internally while keeping the L6387 inputs disconnected from TIM1 alternate function.

Confirmed TIM1 state:

```text
TIM1 clock enabled
TIM1 counter running
TIM1 ARR = 3999
TIM1 CCR1 = 0
TIM1 CCR2 = 0
TIM1 CCR3 = 0
TIM1 CCER = 0
TIM1 BDTR.MOE = 0
```

Expected RTT fields:

```text
drive_safe=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
tim1_counting=1
tim1_arr=3999
tim1_ccr1=0 tim1_ccr2=0 tim1_ccr3=0
tim1_ccer=0
tim1_moe=0
```

Interpretation:

```text
TIM1 is alive internally.
TIM1 is not driving the L6387 inputs.
All phase-drive input pins remain commanded LOW by GPIO.
No MOSFET switching is commanded.
```

## Current Firmware Behavior

```text
button released:
  potentiometer controls STATUS LED blink speed

button held:
  fixed fast blink override

RTT terminal:
  prints live board telemetry, TIM1 readback, drive-pin readback, and timeout state
```

Current RTT output includes fields similar to:

```text
button=0 drive_safe=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0 \
tim1_counting=1 tim1_cnt_a=<raw> tim1_cnt_b=<raw> tim1_arr=3999 \
tim1_ccr1=0 tim1_ccr2=0 tim1_ccr3=0 tim1_ccer=0 tim1_moe=0 \
pot_raw=<raw> pot_pct=<pct> temp_raw=<raw> temp_delta=<delta> \
vbus_raw=<raw> vbus_delta=<delta> \
op1_raw=<raw> op1_delta=<delta> op2_raw=<raw> op2_delta=<delta> \
op3_raw=<raw> op3_delta=<delta> delay_cycles=<value> timeout=0 mode=pot_control
```

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
- motor
- propeller
- phase-to-phase resistor load
```

until PWM, dead-time, gate-driver behavior, current sensing, and protection behavior are deliberately tested.

Current safe-tested features include:

```text
PC6  STATUS LED
PC10 button
PB12 potentiometer ADC
PB14 temperature feedback ADC
PA0  VBUS ADC
PA2/PA6/PB1 raw OPAMP output ADC monitor
TIM1 internal counter only, outputs disabled
Six L6387 input pins held GPIO LOW
RTT over ST-LINK/SWD
```

Bench supply testing performed so far:

```text
USB connected for ST-LINK / RTT
Bench supply connected to ESC input only for VBUS raw monitor
No motor
No propeller
No PWM on drive pins
Current limit around 100 mA during low-voltage VBUS test
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
- print temp_raw and temp_delta over RTT
- confirm temp_raw moves when board is warmed by hand
```

### Stage 5 — Completed

VBUS raw monitor:

```text
- configure PA0 as analog input
- read ADC1_IN1
- print vbus_raw and vbus_delta over RTT
- confirm vbus_raw rises with external bench supply voltage
```

### Stage 6 — Completed

Raw current-feedback / OPAMP output monitor:

```text
- monitor PA2 / OP1_OUT / ADC1_IN3
- monitor PA6 / OP2_OUT / ADC2_IN3
- monitor PB1 / OP3_OUT / ADC1_IN12
- capture approximate zero-current offsets
```

### Stage 7 — Completed

Drive pins safe-low GPIO test:

```text
- configure UH/UL/VH/VL/WH/WL as GPIO outputs
- preload output latches LOW
- force all six drive inputs LOW
- read back GPIO input state
- confirm drive_safe=1
```

### Stage 8 — Completed

TIM1 internal counter test:

```text
- enable TIM1 clock
- configure PSC/ARR/CCR1/CCR2/CCR3
- configure internal PWM mode registers only
- keep CCER=0
- keep BDTR.MOE=0
- keep six drive pins as GPIO LOW
- confirm TIM1 counter is running from register readback
```

### Stage 9 — Next Suggested Step

TIM1 alternate-function setup with outputs still disabled.

Goal:

```text
- switch the six drive pins to TIM1 alternate-function mapping
- keep CCER outputs disabled initially
- keep BDTR.MOE=0
- configure safe idle states before enabling any output path
- confirm no unexpected high state on UH/UL/VH/VL/WH/WL
```

Important note:

```text
This is more sensitive than v0.2.1 because the pins will stop being plain GPIO outputs.
Before writing this step, define CCER polarity, output idle states, BDTR idle behavior, and MOE strategy.
```

### Stage 10 — Future Gate Driver / PWM Output Test

Only after alternate-function idle behavior is understood:

```text
- enable one known-safe low-side or limited PWM path
- use current-limited bench supply
- no motor
- no propeller
- consider measurement with scope before attaching any load
```

### Stage 11 — Future First Motor Experiment

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

The log rate currently follows the main loop rate. Since the potentiometer controls blink delay, it also affects how quickly RTT log lines scroll.

## Git Milestones

Confirmed milestones:

```text
v0.1.6-rtt-adc-live-value
v0.1.7-pb14-temperature-feedback-raw
v0.1.8-pa0-vbus-raw-monitor
v0.1.9-opamp-output-raw-monitor
v0.2.0-drive-pins-safe-low-readback
v0.2.1-tim1-internal-counter-moe-off
```

Recommended next tags:

```text
v0.2.2-tim1-af-moe-off
v0.3.0-first-safe-pwm-output-test
v0.4.0-open-loop-6step-first-spin
```

<!--
Footer
File: README.md
Version: v0.2.1-tim1-internal-counter-moe-off
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->
