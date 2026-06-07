# ================================================================
# File: README.md
# Path: ~/stm32-rust-test/b-g431b-esc1-rust/README.md
# Version: v0.4.0-bemf-observe-openloop
# Purpose: Project notes for STM32G431CB / B-G431B-ESC1 bare-metal
#          Rust ESC bring-up, current monolithic BEMF observe baseline,
#          and next planned split before open-loop sine/SPWM control.
# Target: ST B-G431B-ESC1, STM32G431CB, Cortex-M4F,
#         thumbv7em-none-eabihf
# ================================================================

# B-G431B-ESC1 Rust Bring-up

Rust `no_std` bare-metal bring-up project for the ST B-G431B-ESC1 Discovery kit using the STM32G431CB motor-control MCU.

Current confirmed milestone:

```text
v0.4.0-bemf-observe-openloop
```

Current code state:

```text
Monolithic src/main.rs baseline.
Open-loop six-step PWM motor drive.
BEMF observe instrumentation added.
No closed-loop commutation yet.
No sinusoidal/SPWM yet.
```

Major current milestone:

```text
Small BLDC motor spins open-loop from direct TIM1 complementary-output control.
Floating-phase BEMF path is observable at higher RPM, but not clean enough yet for closed-loop use.
```

The next immediate project step is a **refactor-only file split** of the current working monolithic firmware. After that split builds and behaves the same, the next control experiment will be **open-loop sinusoidal/SPWM**.

---

## Hardware

```text
Board: ST B-G431B-ESC1 Discovery kit
MCU: STM32G431CB
Core: Arm Cortex-M4F
Gate drivers: L6387
MOSFETs: STL180N6F7
Debug/logging: ST-LINK/V2-1, probe-rs, cargo-embed, RTT
Target: thumbv7em-none-eabihf
```

Confirmed probe:

```text
STM32G431CB
chipid: 0x468
flash: 128 KiB
sram: 32 KiB
```

Motor test setup used so far:

```text
Small 2212-class BLDC motor
No prop
Bench supply around 8 V during early spin tests
Current-limited bench supply, initially low and later raised cautiously
Board and motor remained cool during short tests
```

---

## Useful commands

Clean:

```bash
cargo clean
```

Build:

```bash
cargo build --release
```

Flash/run:

```bash
cargo embed --release
```

Attach RTT monitor without reflashing:

```bash
probe-rs attach --chip STM32G431CB target/thumbv7em-none-eabihf/release/b-g431b-esc1-rust
```

Check tree:

```bash
tree
```

Current tree before the split:

```text
.
├── build.rs
├── Cargo.lock
├── Cargo.toml
├── Embed.toml
├── memory.x
├── README.md
└── src
    └── main.rs
```

---

## Confirmed board signals

Basic board I/O:

```text
PC6   STATUS LED
PC10  user button input, active-low dead-man
PB12  potentiometer / ADC1_IN11
PB14  temperature feedback / ADC1_IN5
PA0   VBUS feedback / ADC1_IN1
PA2   OP1_OUT current monitor / ADC1_IN3
PA6   OP2_OUT current monitor / ADC2_IN3
PB1   OP3_OUT current monitor / ADC1_IN12
```

TIM1 drive outputs:

```text
PA8   UH / TIM1_CH1
PC13  UL / TIM1_CH1N
PA9   VH / TIM1_CH2
PA12  VL / TIM1_CH2N
PA10  WH / TIM1_CH3
PB15  WL / TIM1_CH3N
```

Confirmed alternate functions:

```text
PA8   AF6
PA9   AF6
PA10  AF6
PA12  AF6
PB15  AF4
PC13  AF4
```

BEMF detection network currently used for observation:

```text
PB5   GPIO_BEMF control
PA4   BEMF1 / U / OUT1 / ADC2_IN17
PC4   BEMF2 / V / OUT2 / ADC2_IN5
PB11  BEMF3 / W / OUT3 / ADC2_IN14
```

Current BEMF mode:

```text
PB5 held as input / high-Z.
On-board BEMF divider disabled.
Samples are taken during PWM-off timing windows for ground-referenced observation.
```

Bring-up note:

```text
GPIO alternate-function routing alone is not enough for the motor outputs.
TIM1 CCER, CCMR mode fields, BDTR.MOE, complementary polarity, and safe idle behavior all have to agree before the gate-driver inputs behave correctly.
```

---

## Bring-up summary

### 1. Rust firmware / RTT / direct register access

Confirmed:

```text
Rust no_std firmware runs on STM32G431CB.
RTT logging works through probe-rs.
Direct register access with read_volatile/write_volatile is sufficient for this bring-up.
No HAL dependency is required for the current experiments.
```

Learning note:

```text
The bring-up is intentionally close to the metal. RCC, GPIO, ADC, and TIM1 are configured through their registers so each hardware effect can be observed directly in the logs.
```

---

### 2. GPIO / ADC board monitoring

Confirmed:

```text
PC6 LED output works.
PC10 button input works as active-low.
PB12 potentiometer ADC works.
PB14 temperature feedback ADC works.
PA0 VBUS feedback ADC works.
PA2/PA6/PB1 op-amp/current monitor raw ADC readings work.
```

Observed behavior:

```text
Potentiometer controlled LED blink/log rate in earlier versions.
Temperature raw value rises when the ESC board is warmed by hand.
VBUS raw value rises when external ESC power input is raised.
With USB/backfeed only, PA0 VBUS raw sat around the earlier baseline.
With external bench supply raised, VBUS raw rose proportionally.
```

Learning note:

```text
The ADC path is useful as a board-health monitor before it becomes a control input.
VBUS and temperature were used first as sanity/safety signals, not as calibrated physical units.
```

---

### 3. TIM1 / gate-drive command path

Confirmed across earlier versions:

```text
All six drive inputs can be held low.
TIM1 can run internally.
TIM1 alternate-function routing works.
TIM1 CCER and MOE can be configured safely.
Forced-inactive all-low state works after complementary polarity correction.
Each intended gate-driver input can be commanded individually.
Non-target drive inputs stay low during each single-output test.
```

Known good all-off forced-low state from earlier static testing:

```text
expected_ccer=3549
UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

Important finding:

```text
Forced inactive with normal complementary polarity did not make all six drive inputs low.
UH/VH/WH were low, but UL/VL/WL read high.
The corrected all-low forced-inactive state used inverted complementary output polarity.
```

Learning note:

```text
For an advanced-control timer like TIM1, the apparent GPIO pin level depends on timer mode, output enable bits, complementary polarity, idle behavior, and MOE. Reading GPIO IDR during bring-up was useful for confirming that the commanded state reached the physical MCU pins.
```

---

### 4. Bootstrap-aware pulse testing

Confirmed no-motor bootstrap-aware sequences:

```text
UL charge -> all off -> UH command -> all off
VL charge -> all off -> VH command -> all off
WL charge -> all off -> WH command -> all off
```

Confirmed fields during no-motor pulse testing:

```text
state_ok=1
af_ok=1
pins_ok=1
no_phase_overlap=1
active_count_ok=1
tim1_ok=1
cycle_ok=1
```

External VBUS no-motor test:

```text
Motor disconnected.
Bench supply tested up to 12 V.
No unexpected current draw.
Board stayed cold.
VBUS ADC tracked the applied supply.
```

Learning note:

```text
The high-side bootstrap path had to be treated as a real board-level behavior, not just a timer setting. Early tests charged the low side first, inserted all-off deadtime, then commanded the high side.
```

---

### 5. First motor-connected twitch

Confirmed version at that stage:

```text
v0.2.14-button-gated-u-twitch-prep
```

Firmware behavior:

```text
Default state: all outputs off.
Button press ran one short twitch sequence.
After twitch: all outputs off.
Waited for button release before allowing the next twitch.
```

Observed result:

```text
Each button press produced a small physical twitch/jump.
No continuous spin was attempted at that stage.
Board stayed cool.
No abnormal behavior reported.
```

Learning note:

```text
The first real motor movement was intentionally a button-gated single vector. That separated gate-drive correctness from commutation timing and kept the failure energy low.
```

---

### 6. Open-loop six-step spin

Later milestone:

```text
v0.2.17-deadman-fast-ramp-minimal-logging
```

Confirmed behavior:

```text
Small 2212 motor connected.
No prop.
Bench supply around 8 V.
Current limit raised cautiously, around 250 mA during the first useful spin report.
Motor buzzed/hummed but turned consistently all the way around.
Rotation was jittery/not smooth.
ESC and motor stayed cool.
```

Control style:

```text
Open-loop six-step.
One high side PWMing.
One low side held on.
One phase floating.
Button release returns immediately to all-off.
```

Learning note:

```text
Six-step open-loop can prove the timer/gate-driver/motor path, but it produces torque ripple and can slip if the commanded electrical ramp outruns the rotor. The audible buzz during faster tests is expected from six-step torque ripple, open-loop sync behavior, or logging/timing disturbance, not necessarily from the PWM carrier.
```

---

### 7. PWM carrier raised to about 20 kHz

Current TIM1 PWM setup uses:

```text
TIM1_TEST_PSC = 0
TIM1_TEST_ARR = 799
```

With the default HSI16 timer clock assumption:

```text
16 MHz / 800 = about 20 kHz
```

Result:

```text
The PWM carrier itself moved above the main audible range.
Remaining buzz is more likely six-step commutation torque ripple, open-loop slip/catch behavior, or test/log timing effects.
```

Learning note:

```text
PWM carrier frequency and commutation frequency are separate. Raising the carrier to 20 kHz does not remove the lower-frequency torque ripple caused by six-step commutation.
```

---

### 8. BEMF observe added

Current confirmed baseline:

```text
v0.4.0-bemf-observe-openloop
```

Purpose:

```text
Keep the motor drive open-loop.
Add BEMF observation only.
Do not use BEMF to decide commutation yet.
```

Current BEMF behavior:

```text
The code samples the expected floating phase during each six-step vector.
Logs include b0, b1, b2, b3 samples per step.
At low speed or poor sync, readings can be weak or zero.
At higher RPM, BEMF-like readings appear on some phases.
The waveform is not yet clean/reliable enough for closed-loop commutation.
```

Learning note:

```text
Six-step naturally provides a floating phase, which makes BEMF observation practical. Sinusoidal/SPWM usually drives all three phases, so the present floating-phase BEMF method will not directly carry over to the sine test.
```

---

## Current safety behavior

Current firmware keeps these safety assumptions:

```text
No prop.
Low voltage first.
Current-limited bench supply.
Button held = run.
Button released = immediate all-off.
Temperature delta monitored.
ADC timeout monitored.
No same-phase high/low overlap check.
Startup and fault logging retained.
```

Current health-style fields seen in run logs:

```text
health_ok
af_ok
no_phase_overlap
timeout
temp_delta
temp_ok
vbus_raw
vbus_delta
```

Important limitation:

```text
health_ok does not prove the rotor is synchronized.
It only proves the deterministic electrical/safety checks passed.
```

---

## Current status

```text
The project has moved from static pin proof, to first twitch, to open-loop six-step spin, to BEMF observation.
The current monolithic v0.4.0 firmware is a useful baseline before restructuring.
```

Current working baseline:

```text
src/main.rs
v0.4.0-bemf-observe-openloop
```

Current repository step completed before this README update:

```text
Cargo.toml was updated to match v0.4.0-bemf-observe-openloop.
The monolithic baseline was pushed before the split work begins.
```

---

## What is still not confirmed

```text
actual MOSFET gate voltage
actual high-side bootstrap voltage under fast PWM load
phase-node switching waveform
calibrated motor current
dead-time margin under real PWM with higher current
reliable sensorless zero-cross timing
closed-loop commutation stability
open-loop sine/SPWM behavior on this board
thermal behavior under repeated or longer drive
```

A scope or current probe would be useful before aggressive PWM, higher bus voltage, or closed-loop experiments.

---

## Next immediate step: refactor-only split

The next step is not a behavior change.

Goal:

```text
Split the current working monolithic v0.4.0 main.rs into modules.
Preserve existing six-step + BEMF observe behavior.
Build and test the split version the same way as the monolithic version.
```

Proposed split:

```text
src/main.rs        startup, main loop, run selection
src/regs.rs        raw register addresses and bit constants
src/gpio.rs        pin modes, AF setup, LED, button, drive pin readback
src/adc.rs         ADC1/ADC2 setup and board monitor reads
src/tim1.rs        TIM1 setup, PWM/CCER/BDTR helpers
src/drive.rs       DriveState, six-step states, apply_state/apply_pwm_vector
src/safety.rs      overlap/temp/button/health helpers
src/bemf.rs        floating-phase BEMF observe helpers
src/sixstep.rs     current open-loop six-step ramp
src/log.rs         RTT log formatting helpers
```

Expected version for the split milestone:

```text
v0.4.1-split-same-behavior
```

Refactor rule:

```text
No sine code in the first split.
No closed-loop behavior in the first split.
No pin remapping in the first split.
No safety behavior removal in the first split.
```

Learning-note rule for rewritten files:

```text
Each rewritten module should include practical bring-up notes explaining what was learned from the board and why that subsystem is structured the way it is.
```

---

## Next control experiment after split: open-loop sine/SPWM

After the split version builds and behaves the same, the next experimental branch is:

```text
v0.5.0-openloop-sine-spwm
```

Goal:

```text
Test smoother open-loop sinusoidal phase drive before attempting FOC.
```

Expected control style:

```text
TIM1 carrier remains around 20 kHz.
U/V/W PWM duties are updated from a sine table.
Three phases are 120 electrical degrees apart.
Open-loop electrical angle advances with a ramp.
Amplitude ramps cautiously.
Button dead-man remains active.
VBUS/temp monitoring remains active.
BEMF observe disabled for the first sine test.
```

Important tradeoff:

```text
Six-step leaves one phase floating, which helps BEMF observation.
Sinusoidal/SPWM drives all three phases, so the current floating-phase BEMF method is not the right first feedback path.
```

Later path toward FOC:

```text
GPIO / ADC / TIM1 bring-up
-> static drive-vector proof
-> first motor twitch
-> open-loop six-step
-> six-step + BEMF observe
-> refactor-only split
-> open-loop sine/SPWM
-> later current sensing / rotor-angle estimate / FOC experiments
```

---

## Commit/tag convention

Use version tags matching the tested firmware milestone, for example:

```text
v0.4.0-bemf-observe-openloop
v0.4.1-split-same-behavior
v0.5.0-openloop-sine-spwm
```

Recommended baseline commit before split:

```bash
git status
git add Cargo.toml README.md src/main.rs
git commit -m "Save monolithic BEMF observe open-loop baseline"
git tag v0.4.0-bemf-observe-openloop
git push
git push origin v0.4.0-bemf-observe-openloop
```

For the README-only update after the baseline push:

```bash
git status
git add README.md
git commit -m "Update README for BEMF observe baseline and split plan"
git push
```

---

## Status summary

Passed milestones:

```text
v0.2.6-fix2 UL low-side passed
v0.2.7 VL low-side passed
v0.2.8 WL low-side passed

v0.2.9 UH high-side input-command passed
v0.2.10 VH high-side input-command passed
v0.2.11 WH high-side input-command passed

v0.2.12 U-phase bootstrap-aware no-motor pulse passed
v0.2.13 all-phases bootstrap-aware no-motor pulse passed

v0.2.14 button-gated UH+VL first motor twitch passed
v0.2.17 dead-man open-loop slow spin passed

v0.3.x PWM six-step ramp with about 20 kHz carrier tested
v0.4.0 BEMF observe added while keeping loop open
```

Current milestone:

```text
v0.4.0-bemf-observe-openloop
```

Next milestone:

```text
v0.4.1-split-same-behavior
```

Next control milestone after split:

```text
v0.5.0-openloop-sine-spwm
```

---

# ================================================================
# Footer
# File: README.md
# Version: v0.4.0-bemf-observe-openloop
# Created: 2026-06-07
# Generated timestamp: 2026-06-07
# ================================================================
