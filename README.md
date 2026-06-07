<!--
File: README.md
Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
Version: v0.2.3-fix1-tim1-ccer-enabled-moe-off
Purpose: Project notes and bring-up roadmap for Rust firmware on the B-G431B-ESC1 / STM32G431CB ESC board
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->

# STM32 Rust Test — B-G431B-ESC1 ESC Bring-up

Rust firmware bring-up project for the **ST B-G431B-ESC1 Discovery ESC board**.

The goal is to learn the board step by step, starting with safe GPIO and ADC tests, then progressing toward TIM1 setup, gate-driver bring-up, and eventually spinning a small BLDC motor.

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
v0.2.3-fix1-tim1-ccer-enabled-moe-off
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
- six L6387 drive pins moved to TIM1 alternate-function mode
- TIM1 internal counter configured and running
- TIM1 ARR = 3999
- TIM1 CCR1/CCR2/CCR3 = 0
- TIM1 CCER = 1365, enabling CH1/1N/2/2N/3/3N internally
- TIM1 BDTR.MOE = 0, so main output enable remains OFF
- TIM1 BDTR.OSSI = 1
- TIM1 CR2 output idle-state bits remain 0
- no intentional gate switching yet
```

Confirmed key RTT fields from `v0.2.3-fix1`:

```text
safety_ok=1
af_ok=1
tim1_counting=1
tim1_ccer=1365
tim1_ccer_expected=1
tim1_moe=0
tim1_ossi=1
tim1_ccr1=0
tim1_ccr2=0
tim1_ccr3=0
UH_pin=0 UL_pin=0 VH_pin=0 VL_pin=0 WH_pin=0 WL_pin=0
```

Interpretation:

```text
TIM1 owns the six drive pins through alternate function.
TIM1 channel output-enable bits are configured in CCER.
TIM1 master output enable remains OFF.
Duty registers remain zero.
No motor output is intentionally commanded yet.
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
op2_raw ≈ 1970–2005
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

Current firmware pin function state:

```text
PA8  UH = TIM1_CH1  alternate function AF6
PC13 UL = TIM1_CH1N alternate function AF4
PA9  VH = TIM1_CH2  alternate function AF6
PA12 VL = TIM1_CH2N alternate function AF6
PA10 WH = TIM1_CH3  alternate function AF6
PB15 WL = TIM1_CH3N alternate function AF4
```

`v0.2.2` confirmed alternate-function setup:

```text
af_ok=1
UH_af=6
VH_af=6
WH_af=6
VL_af=6
WL_af=4
UL_af=4
tim1_ccer=0
tim1_moe=0
```

`v0.2.3-fix1` confirmed CCER enabled while MOE remains off:

```text
af_ok=1
tim1_counting=1
tim1_ccer=1365
tim1_ccer_expected=1
tim1_moe=0
tim1_ossi=1
tim1_ccr1=0
tim1_ccr2=0
tim1_ccr3=0
safety_ok=1
```

## TIM1 Safety Progression

### `v0.2.0-drive-pins-safe-low-readback`

```text
- configured UH/UL/VH/VL/WH/WL as GPIO outputs
- forced all six LOW
- confirmed drive_safe=1 from GPIO readback
```

### `v0.2.1-tim1-internal-counter-moe-off`

```text
- enabled TIM1 clock
- configured TIM1 PSC/ARR/CCR1/CCR2/CCR3
- configured internal PWM mode registers
- kept CCER=0
- kept BDTR.MOE=0
- kept drive pins as GPIO LOW
- confirmed TIM1 counter running
```

### `v0.2.2-tim1-af-pins-moe-off`

```text
- moved six drive pins to TIM1 alternate-function mode
- confirmed AF register mapping
- kept CCER=0
- kept BDTR.MOE=0
- kept CCR1/2/3=0
```

### `v0.2.3-fix1-tim1-ccer-enabled-moe-off`

```text
- kept drive pins in TIM1 alternate-function mode
- enabled TIM1 CCER bits for CH1/1N/2/2N/3/3N
- kept BDTR.MOE=0
- kept CCR1/2/3=0
- set OSSI=1
- kept CR2 idle-state bits at 0
- confirmed safety_ok=1
```

Important note:

```text
Even though CCER is now enabled, BDTR.MOE remains OFF.
This is still a no-intentional-gate-switching firmware stage.
Do not connect the motor yet for this milestone.
```

## Current Firmware Behavior

```text
button released:
  potentiometer controls STATUS LED blink speed

button held:
  fixed fast blink override

RTT terminal:
  prints live board telemetry, TIM1 readback, AF mapping, safety state, and timeout state
```

Current RTT output includes fields similar to:

```text
button=0 safety_ok=1 af_ok=1 tim1_counting=1 \
tim1_ccer=1365 tim1_ccer_expected=1 tim1_moe=0 tim1_ossi=1 \
tim1_ccr1=0 tim1_ccr2=0 tim1_ccr3=0 \
UH_pin=0 UL_pin=0 VH_pin=0 VL_pin=0 WH_pin=0 WL_pin=0 \
UH_af=6 UL_af=4 VH_af=6 VL_af=6 WH_af=6 WL_af=4 \
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
TIM1 counter
TIM1 alternate-function drive-pin mapping
TIM1 CCER output-enable register path
TIM1 BDTR.MOE held OFF
RTT over ST-LINK/SWD
```

Bench supply testing performed so far:

```text
USB connected for ST-LINK / RTT
Bench supply connected to ESC input only for VBUS raw monitor
No motor
No propeller
No PWM intentionally output to drive a motor
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

### Stage 9 — Completed

TIM1 alternate-function setup with outputs disabled:

```text
- switch the six drive pins to TIM1 alternate-function mapping
- confirm AF register values
- keep CCER=0
- keep BDTR.MOE=0
- keep CCR1/2/3=0
```

### Stage 10 — Completed

TIM1 CCER enabled with MOE still off:

```text
- keep the six drive pins in TIM1 alternate-function mode
- enable CH1/1N/2/2N/3/3N bits in CCER
- keep BDTR.MOE=0
- keep CCR1/2/3=0
- set OSSI=1
- confirm safety_ok=1
```

### Stage 11 — Next Suggested Step

Forced inactive / MOE-on safety test.

Goal:

```text
- configure TIM1 channels into forced inactive / forced low mode
- keep duty effectively off
- enable BDTR.MOE only after forced inactive mode is confirmed
- no motor
- no propeller
- use current-limited bench supply if powering VBUS
- verify no current jump and telemetry remains stable
```

This is the first step where `MOE=1` may be tested, so it must be treated as more sensitive than the previous register-only stages.

### Stage 12 — Future First Controlled Output Test

Only after forced-inactive `MOE=1` behavior is understood:

```text
- consider a single controlled low-side output path
- very low duty or forced-output test
- current-limited bench supply
- no motor initially if possible
- no propeller
- prefer oscilloscope/meter confirmation when available
```

### Stage 13 — Future First Motor Experiment

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
v0.2.2-tim1-af-pins-moe-off
v0.2.3-fix1-tim1-ccer-enabled-moe-off
```

Recommended next tags:

```text
v0.2.4-tim1-forced-inactive-moe-on
v0.3.0-first-safe-output-test
v0.4.0-open-loop-6step-first-spin
```

<!--
Footer
File: README.md
Version: v0.2.3-fix1-tim1-ccer-enabled-moe-off
Created: 2026-06-07
Generated timestamp: 2026-06-07
-->
