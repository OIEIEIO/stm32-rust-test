<!--
File: README.md
Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
Version: v0.1.9-opamp-output-raw-monitor
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
  - Has its own LEDs and activity indication
  - Not the MCU being programmed by this project

ESC / target section
  - Contains the STM32G431CB target MCU
  - Contains motor-control hardware, gate drivers, MOSFETs, current feedback, VBUS feedback, and board I/O
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
v0.1.9-opamp-output-raw-monitor
```

Confirmed working:

```text
- ST-LINK detection
- probe-rs flashing
- cargo-embed RTT terminal output
- probe-rs attach without reflashing
- Rust no_std firmware boot
- linker/memory.x setup
- PC6 STATUS LED output
- PC10 user button input
- PB12 potentiometer ADC input
- PB14 temperature feedback ADC input
- PA0 VBUS feedback ADC input
- PA2 OP1 output raw ADC monitor
- PA6 OP2 output raw ADC monitor
- PB1 OP3 output raw ADC monitor
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

Current feedback schematic interpretation:

```text
Current feedback phase 1:
  PA1 = Curr_fdbk1_OPamp+
  PA2 = OP1_OUT
  PA3 = Curr_fdbk1_OPamp-

Current feedback phase 2:
  PA7 = Curr_fdbk2_OPamp+
  PA6 = OP2_OUT
  PA5 = Curr_fdbk2_OPamp-

Current feedback phase 3:
  PB0 = Curr_fdbk3_OPamp+
  PB1 = OP3_OUT
  PB2 = Curr_fdbk3_OPamp-
```

## Observed Signal Behavior

### Potentiometer

```text
pot far right  = low ADC value, near 0
pot far left   = high ADC value, up to 4095
```

Current firmware behavior:

```text
button released:
  potentiometer controls STATUS LED blink speed

button held:
  fixed fast blink override
```

### Temperature Feedback

PB14 temperature feedback is confirmed live:

```text
- temp_raw rises when the ESC board is warmed by hand
- temp_raw falls back toward ambient when released
- values are raw ADC only, not Celsius
```

### VBUS Feedback

PA0 VBUS feedback is confirmed live and roughly proportional to ESC input voltage.

Observed bench test:

```text
USB / ESC input seen:  about 4.73 V  -> vbus_raw about 577
Bench supply:          about 10.0 V  -> vbus_raw about 1214
```

Approximate bench-derived scale:

```text
vbus_volts ≈ vbus_raw / 121.5
```

This is only a rough working estimate until the actual VBUS divider values are confirmed from the schematic.

### Current Feedback / OPAMP Output Raw Monitor

With no motor current and no PWM, the raw OPAMP output channels read stable zero-current offsets.

Observed zero-current raw values:

```text
op1_raw ≈ 1598–1615
op2_raw ≈ 1997–1999
op3_raw ≈ 1518–1524
timeout = 0
```

Interpretation:

```text
- ADC reads are valid
- OPAMP output monitor path is alive
- these are zero-current offset/bias values
- these are not load-current measurements yet
```

Do not connect resistors between motor phase outputs for this firmware. There is no PWM, no gate-driver enable, and no controlled current path yet.

## RTT Output

Current firmware prints live board telemetry over RTT/SWD.

Typical output fields:

```text
button=<0/1>
pot_raw=<raw ADC>
pot_pct=<0..100>
temp_raw=<raw ADC>
temp_delta=<raw-startup>
vbus_raw=<raw ADC>
vbus_delta=<raw-startup>
op1_raw=<raw ADC>
op1_delta=<raw-startup>
op2_raw=<raw ADC>
op2_delta=<raw-startup>
op3_raw=<raw ADC>
op3_delta=<raw-startup>
delay_cycles=<value>
timeout=<0/1>
mode=<pot_control/button_fast>
```

RTT output is carried through the ST-LINK debug connection, not UART. The large LED near the USB connector may show activity while probe-rs or cargo-embed is attached and reading RTT data.

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
- resistor between phase outputs
```

Do not intentionally load phase outputs until PWM, gate-driver enable, fault inputs, current sensing, and protection behavior are understood.

A bench supply can be used for VBUS feedback testing only when:

```text
- polarity is confirmed
- current limit is low
- no motor is attached
- firmware does not enable PWM or gate drivers
```

Current safe-tested features only touch:

```text
PC6   STATUS LED
PC10  button
PB12  potentiometer ADC
PB14  temperature feedback ADC
PA0   VBUS feedback ADC
PA2   OP1_OUT ADC monitor
PA6   OP2_OUT ADC monitor
PB1   OP3_OUT ADC monitor
RTT over ST-LINK/SWD
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

Temperature feedback monitor:

```text
- configure PB14 as analog input
- read ADC1_IN5
- confirm raw value changes with hand warming
- leave value as raw ADC, not Celsius
```

### Stage 5 — Completed

VBUS feedback monitor:

```text
- configure PA0 as analog input
- read ADC1_IN1
- confirm raw value changes with bench supply input voltage
- rough observed scale: vbus_volts ≈ vbus_raw / 121.5
```

### Stage 6 — Completed

Current-feedback raw OPAMP output monitor:

```text
- configure PA2 OP1_OUT raw monitor
- configure PA6 OP2_OUT raw monitor using ADC2
- configure PB1 OP3_OUT raw monitor
- confirm stable zero-current offsets at no load
```

### Stage 7 — Next Suggested Step

Inspect gate-driver, fault, shutdown, and enable pins before any PWM work.

Candidate schematic targets:

```text
- gate-driver enable / shutdown pins
- fault pins
- overcurrent / protection pins
- brake / disable logic
- TIM1 phase PWM pins
```

Goal:

```text
RTT output shows driver/fault/protection state before any motor switching.
```

### Stage 8 — Timer Bring-up, No Power Stage

Configure motor PWM timer without enabling the gate drivers.

Likely area of work:

```text
TIM1 complementary PWM
UH/VH/WH high-side control signals
UL/VL/WL low-side control signals
dead-time setup
PWM frequency setup
safe default duty cycle
all outputs disabled at startup
gate drivers disabled
```

Safety state:

```text
gate drivers disabled
no motor attached
no phase load attached
```

### Stage 9 — Gate Driver / Fault Bring-up

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

### Stage 10 — Controlled Resistor-Load Current Test

A future current-feedback test should use a controlled current path, not a resistor across phase outputs.

Possible future concept:

```text
VBUS+ -> power resistor -> one phase output -> commanded low-side MOSFET -> current shunt -> GND
```

Required first:

```text
- gate-driver enable/fault pins understood
- PWM outputs understood
- one low-side MOSFET can be commanded safely
- current-limited bench supply
- appropriate power resistor
- no motor
- no propeller
```

### Stage 11 — First Motor Experiment

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

The ST-LINK section remains intact after flashing this project because the firmware is written to the STM32G431CB target MCU on the ESC section.

The VBUS raw monitor shows the board input/DC bus sense, not a calibrated voltage yet.

The current-feedback raw monitor currently proves zero-current offsets. It does not prove load-current response yet.

## Git Milestones

Current baseline commit target:

```text
v0.1.9 add raw opamp output monitor
```

Recommended future tags:

```text
v0.1.6-rtt-adc-live-value
v0.1.7-pb14-temperature-feedback-raw
v0.1.8-pa0-vbus-raw-monitor
v0.1.9-opamp-output-raw-monitor
v0.2.0-driver-fault-shutdown-monitor
v0.3.0-tim1-pwm-disabled-output-test
v0.4.0-gate-driver-safe-enable-test
v0.5.0-open-loop-6step-first-spin
```

<!--
Footer
File: README.md
Version: v0.1.9-opamp-output-raw-monitor
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->
