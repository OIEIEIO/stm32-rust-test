# B-G431B-ESC1 Rust Bring-up

Rust bare-metal motor-control bring-up project for the ST **B-G431B-ESC1** Discovery kit using the **STM32G431CB** motor-control MCU.

Current confirmed checkpoint:

```text
v0.5.24-warning-cleanup
```

Current state:

```text
The split-module firmware builds cleanly with cargo build --release.
Current build result: 0 errors, 0 warnings.

The active runtime path is open-loop sine/SPWM.
The retained six-step path remains available as the BEMF-observe baseline.

TIM1_CH4-triggered injected ADC sampling is working for the current FOC-prep pair:
  ADC1 injected rank 1 = OPAMP1 external output
  ADC2 injected rank 1 = OPAMP2 external output

OPAMP3/VOPAMP3 internal routing has been brought up as a regular ADC diagnostic path:
  OPAMP3/VOPAMP3 regular diagnostic = ADC2_IN18

OPAMP3/VOPAMP3 is not yet part of the injected ADC pair.
No sector-switched current reconstruction has been added yet.
No Clarke/Park, current loop, SVPWM, or closed-loop FOC has been added yet.
```

---

## Hardware

```text
Board: ST B-G431B-ESC1 Discovery kit
MCU: STM32G431CB
Gate drivers: L6387
MOSFETs: STL180N6F7
Debug/logging: ST-LINK/V2-1, probe-rs, RTT
Target: thumbv7em-none-eabihf
```

Confirmed probe:

```text
STM32G431CB
chipid: 0x468
flash: 128 KiB
sram: 32 KiB
```

Current motor-test setup:

```text
Motor: 2212 BLDC, 900 KV
Prop: none
Bench voltage: about 10 V during latest sine/SPWM testing
Bench current-limit setting: bench-limited during testing
Observed current during successful sine/SPWM run: about 400 mA
Control mode: button-held dead-man operation
Firmware mode: open-loop sine/SPWM, 96-step sine table, fast inner CCR update loop
```

Current safety practice:

```text
No prop.
Low voltage.
Bench current limit enabled.
Button release stops the run.
Board and motor are checked for abnormal heating during testing.
VBUS and temperature are logged through ADC snapshots.
```

---

## Useful commands

Build:

```bash
cargo build --release
```

Flash/run:

```bash
cargo run --release
```

Attach RTT monitor without reflashing:

```bash
probe-rs attach --chip STM32G431CB target/thumbv7em-none-eabihf/release/b-g431b-esc1-rust
```

Normal workflow:

```text
cargo run --release
```

`cargo embed` is not required for the current workflow. `cargo run --release` has been the clean path for build/flash/run testing.

Clean build artifacts:

```bash
cargo clean
cargo build --release
```

Check repo state before commit:

```bash
git status
git diff --stat
```

Current clean checkpoint commit example:

```bash
git add Cargo.toml Cargo.lock \
  src/adc.rs \
  src/tim1.rs \
  src/regs.rs \
  src/opamp.rs \
  src/current_sense.rs \
  src/log.rs \
  src/main.rs \
  src/sine.rs

git commit -m "Clean OP3 VOPAMP3 diagnostic warnings"
git push
```

Optional tag:

```bash
git tag -a v0.5.24-warning-cleanup -m "Clean OP3 VOPAMP3 diagnostic warnings"
git push origin v0.5.24-warning-cleanup
```

---

## Current project tree

```text
.
├── build.rs
├── Cargo.lock
├── Cargo.toml
├── Embed.toml
├── memory.x
├── README.md
└── src
    ├── adc.rs
    ├── bemf.rs
    ├── current_sense.rs
    ├── drive.rs
    ├── gpio.rs
    ├── log.rs
    ├── main.rs
    ├── opamp.rs
    ├── regs.rs
    ├── safety.rs
    ├── sine.rs
    ├── sixstep.rs
    └── tim1.rs
```

Module responsibilities:

```text
src/main.rs          startup sequence, top-level initialization, main control loop
src/regs.rs          register bases, register pointers, bit masks, board constants
src/gpio.rs          generic GPIO helpers, LED, button input
src/adc.rs           ADC setup, channel selection, raw ADC reads, board monitor snapshot
src/opamp.rs         OPAMP1/2/3 setup and OP3/VOPAMP3 internal-route enable/readback
src/current_sense.rs zero-offset capture, current-sense observation, injected OP1/OP2 sampling, OP3/VOPAMP3 regular diagnostic read
src/tim1.rs          TIM1 setup, PWM vectors, sine PWM helpers, CH4 trigger setup, CCER/CCMR/BDTR readback
src/drive.rs         DriveState, expected pin/CCER states, drive-pin readback, overlap checks
src/bemf.rs          BEMF pin setup, floating-phase selection, floating-phase ADC sampling
src/sixstep.rs       open-loop six-step ramp runner, retained as baseline
src/sine.rs          open-loop sine/SPWM runner, current active experiment
src/log.rs           structured RTT status and current-sense diagnostic logging
src/safety.rs        delay helpers and button-held dead-man helper
```

Design principle:

```text
Keep control strategies separated:
sixstep.rs for six-step tests
sine.rs for open-loop sinusoidal/SPWM tests
current_sense.rs for current-sense observation and FOC-prep sampling
foc.rs later for current-oriented control
```

---

## Confirmed board signals

General board I/O:

```text
PC6   STATUS LED
PC10  user button input, active-low
PB12  potentiometer / ADC1_IN11
PB14  temperature feedback / ADC1_IN5
PA0   VBUS feedback / ADC1_IN1
```

Current-sense / OPAMP paths:

```text
OPAMP1 VINP0 PA1 -> VOUT PA2 -> ADC1_IN3
OPAMP2 VINP0 PA7 -> VOUT PA6 -> ADC2_IN3
OPAMP3 VINP0 PB0 -> VOUT PB1 -> ADC1_IN12
OPAMP3 internal VOPAMP3 diagnostic -> ADC2_IN18
```

Current FOC-prep ADC model:

```text
TIM1_CH4 center trigger:
  ADC1 injected = OPAMP1 external VOUT / ADC1_IN3
  ADC2 injected = OPAMP2 external VOUT / ADC2_IN3

OPAMP3/VOPAMP3:
  sampled separately as a regular ADC2 diagnostic read on ADC2_IN18
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

BEMF sense network used for six-step observation:

```text
PB5   GPIO_BEMF control

PA4   BEMF1 / U / OUT1 / ADC2_IN17
PC4   BEMF2 / V / OUT2 / ADC2_IN5
PB11  BEMF3 / W / OUT3 / ADC2_IN14
```

Current BEMF configuration:

```text
BEMF is retained for six-step observation.
BEMF is not used during sine/SPWM because all three phases are driven.
No commutation decision uses BEMF yet.
```

---

## Current firmware behavior

### Startup

Startup does the following:

```text
Initializes RTT logging.
Enables GPIO and ADC clocks.
Configures board monitor analog pins.
Configures TIM1 base PWM timing.
Configures TIM1 drive pins and alternate functions.
Forces the bridge to IdleAllOff.
Configures OPAMP1/2/3.
Enables OPAMP3 internal VOPAMP3 diagnostic routing.
Configures ADC1 and ADC2.
Configures current-sense ADC sample timing.
Configures TIM1_CH4 as the injected ADC trigger source.
Captures current-sense zero offsets.
Logs startup status and expected signal routes.
```

Startup is expected to report:

```text
startup_ok=1
opamp_ok=1
op3v_internal_ok=1
cs_offset_ok=1
op3v_regular_offset_ok=1
af_ok=1
pins_ok=1
no_phase_overlap=1
tim1_ok=1
UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

### Idle

Button released:

```text
Drive is held in IdleAllOff.
Status LED blinks.
Periodic idle state logs are printed.
Current-sense raw/zero/delta fields are logged.
```

### Run

Button held:

```text
Bootstrap precharge sequence runs.
Sine alignment runs.
Open-loop sine/SPWM ramp runs.
Release button to stop immediately.
Run stops at max_steps if button remains held long enough.
```

Typical successful stop reason:

```text
sine_run_stop run=1 cycle_ok=1 sine_steps=32000 reason=max_steps
```

---

## Current logging nomenclature

The current logs intentionally separate injected samples from regular ADC diagnostics.

Injected OP1/OP2 path:

```text
injected_pair=adc1_op1_adc2_op2
op1_jraw=...
op1_delta=...
op2_jraw=...
op2_delta=...
jeoc1=1
jeos1=1
jeoc2=1
jeos2=1
to=0
rail=0
ok=1
```

OP3/VOPAMP3 regular diagnostic path:

```text
op3v_regular_path=adc2_in18
op3v_regular_raw=...
op3v_regular_delta=...
op3v_regular_valid=1
```

Important naming rule:

```text
op1_jraw / op2_jraw are injected ADC results.
op3v_regular_raw is not injected yet.
Do not rename op1/op2/op3v to ia/ib/ic until phase mapping and current reconstruction are confirmed.
```

Compact sector summary naming:

```text
focsum_a / focsum_b
s0_samples
s0_ok_samples
s0_op1_avg
s0_op2_avg
s0_op3v_regular_samples
s0_op3v_regular_avg
s0_isum2_avg
```

---

## Bring-up summary

### 1. Rust no_std firmware baseline

Confirmed:

```text
no_std / no_main firmware works.
cortex-m-rt entry works.
RTT logging works through probe-rs.
Direct register access through read_volatile/write_volatile works.
panic-halt is used.
```

Bring-up lesson:

```text
This project is intentionally close to the metal. Peripheral behavior is learned and verified directly through RCC, GPIO, ADC, TIM1, and board-level signal readbacks instead of hiding the bring-up behind a high-level motor-control framework.
```

---

### 2. GPIO and dead-man control

Confirmed:

```text
PC6 LED output works.
PC10 button input works as active-low.
Button-held operation works as a dead-man control.
Button release returns the drive to all-off.
```

Bring-up lesson:

```text
The button is treated as a safety input, not just a user command. Any drive routine must be written so release of the button exits to IdleAllOff quickly.
```

---

### 3. ADC board monitoring

Confirmed:

```text
PB12 potentiometer ADC works.
PB14 temperature feedback ADC works.
PA0 VBUS feedback ADC works.
PA2 / PA6 / PB1 OPAMP external-output raw ADC readings work.
ADC2_IN18 OPAMP3/VOPAMP3 regular diagnostic reading works.
```

Observed behavior:

```text
Potentiometer controlled LED blink/log rate in earlier tests.
Temperature raw value rises when the ESC board is warmed by hand.
VBUS raw value rises when external ESC power input is raised.
With USB-only/backfeed, PA0 VBUS raw sits around the lower baseline.
With bench supply raised, VBUS raw rises proportionally.
```

Bring-up lesson:

```text
VBUS and temperature monitoring are useful safety signals even before calibrated current sensing exists. They are kept in the run logs so unsafe trends can be caught during early motor-control experiments.
```

---

### 4. TIM1 and gate-drive output bring-up

Confirmed:

```text
TIM1 clock enable works.
TIM1 counter runs.
TIM1 ARR/PSC setup works.
TIM1 CCER command states work.
TIM1 CCMR forced-active / forced-inactive modes work.
TIM1 BDTR.MOE must be set before advanced-control outputs drive.
TIM1 complementary output routing works after polarity/state correction.
TIM1_CH4 internal compare/reference event is preserved while CH3 sine PWM mode is rewritten.
```

Important all-off finding:

```text
Forced inactive with normal complementary polarity did not produce all six drive inputs low.
UH/VH/WH were low, but UL/VL/WL read high.
The corrected all-off forced-low state uses the confirmed CCER/polarity setup.
```

Known-good all-off forced-low state:

```text
expected_ccer=3549
UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

Bring-up lesson:

```text
GPIO alternate-function setup alone is not enough. TIM1 advanced-control behavior depends on CCER, CCMR, BDTR, MOE, polarity, off-state handling, and CH4 preservation while updating CH3 mode. Every drive state needs readback checks.
```

---

### 5. Six-step baseline

Six-step baseline:

```text
Open-loop six-step
One high side PWM
One low side held on
One phase floating
Button held to run
Button released to stop
```

Observed behavior:

```text
The motor can rotate continuously in open loop.
At low current it buzzed/hummed and turned slowly/jittery.
Raising the bench current limit improved the ability to follow the commutation.
At higher commanded speed, BEMF readings became more visible.
Board and motor stayed cool in reported tests.
```

Split-validation motor test:

```text
Milestone: v0.4.1-split-same-behavior
Motor: 2212 900 KV BLDC
Prop: none
Bench voltage: 10 V
Bench current-limit setting: 1.5 A
Observed current during run: about 300 mA
Run result: completed programmed vector sequence
Stop reason: max_steps
No reported abnormal heating
```

Representative run result:

```text
run_stop run=2 cycle_ok=1 vector_steps=1000 button=1 reason=max_steps
```

Bring-up lesson:

```text
Six-step is useful for first motion because one phase naturally floats and can be sampled for BEMF. It is also mechanically rough, especially during open-loop startup and low-speed operation.
```

---

### 6. BEMF observation

Current BEMF mode:

```text
Observe only
No closed-loop commutation
Floating phase selected from the six-step vector
Four ADC samples logged as b0..b3
```

Observed BEMF pattern from the current test setup:

```text
The floating-phase BEMF log is consistent throughout the run.
Every other vector line can show zero readings.
The following vector line for that phase can show the expected nonzero readings.
This repeating pattern appears consistent, not random.
All three phases have shown nonzero readings in their corresponding floating-phase positions.
```

Current interpretation:

```text
The BEMF inputs appear alive.
The alternating zero/nonzero pattern is being preserved as raw logged information.
No closed-loop commutation uses this data yet.
No additional interpreted labels have been added to the log.
```

Bring-up lesson:

```text
At this stage, preserving the exact vector, floating phase, and raw b0..b3 samples is more useful than replacing them with interpreted labels. The raw pattern is part of the evidence needed before any zero-cross or observer logic is added.
```

---

### 7. Open-loop sine/SPWM milestone

Initial sine/SPWM milestone:

```text
v0.5.2-openloop-sine-96-fast-loop
```

Goal:

```text
Test smoother open-loop sinusoidal phase-voltage control while keeping the same dead-man-button safety model.
```

Current sine/SPWM behavior:

```text
Button released: outputs disabled / all phases off
Button held: bootstrap precharge -> sine alignment -> open-loop sine/SPWM ramp
Button released during run: outputs return to all-off
```

Implementation summary:

```text
TIM1 complementary outputs are enabled for CH1/CH1N, CH2/CH2N, and CH3/CH3N.
U/V/W are driven 120 electrical degrees apart.
The 96-step sine table made the motor much quieter and smoother.
The fast inner loop updates CCR1/CCR2/CCR3 directly.
Slow health/readback/logging runs periodically instead of every sine sample.
```

Current observed sine/SPWM bench result:

```text
Motor spun up smoothly to a useful RPM.
Rotation was much quieter than six-step.
The 96-step fast-loop version produced only a slight high-pitched noise.
Rotation was smooth and consistent on the bench.
Observed draw was about 400 mA.
No abnormal heating was reported during the successful test.
```

Important sine/SPWM limitation:

```text
This is still open-loop.
Rotor angle is not measured.
The code does not know whether the rotor is exactly aligned to the commanded electrical field.
BEMF observation is not meaningful in this mode because all three phases are driven.
This is not FOC.
```

Bring-up lesson:

```text
The major improvement came from separating fast waveform generation from slow diagnostics. The 96-step sine table made the waveform quiet, but the per-step health/readback loop dominated timing until diagnostics were moved out of the fast path.
```

---

### 8. TIM1_CH4 injected ADC FOC-prep sampling

Confirmed:

```text
TIM1_CH4 compare/reference setup works.
TIM1_CCMR2 CH4 OCREF config is preserved while CH3 sine PWM mode is rewritten.
ADC1 injected conversion completes from the TIM1_CH4 trigger.
ADC2 injected conversion completes from the TIM1_CH4 trigger.
ADC1/ADC2 JEOC and JEOS flags complete.
Timeout is zero during successful samples.
Rail flag is zero during successful samples.
```

Representative successful fields:

```text
ok=1
jeoc1=1
jeos1=1
jeoc2=1
jeos2=1
to=0
rail=0
```

Current injected pair:

```text
ADC1 injected rank 1 = OPAMP1 external VOUT / ADC1_IN3
ADC2 injected rank 1 = OPAMP2 external VOUT / ADC2_IN3
```

Purpose:

```text
This is a FOC-prep timing proof.
The goal is to prove deterministic ADC sampling tied to a known PWM timing point.
It is not yet a current loop.
```

---

### 9. OPAMP3/VOPAMP3 diagnostic bring-up

Current checkpoint:

```text
v0.5.24-warning-cleanup
```

Confirmed:

```text
OPAMP1/2/3 are enabled.
OPAMP1/2/3 external VOUT ADC paths are present.
OPAMP3 internal VOPAMP3 routing is enabled.
ADC2_IN18 returns valid OPAMP3/VOPAMP3 diagnostic samples.
The OP3/VOPAMP3 diagnostic path has valid zero-offset capture.
The logs clearly label OP3/VOPAMP3 as regular ADC diagnostic data.
```

Representative fields from successful runtime logs:

```text
op3v_regular_path=adc2_in18
op3v_regular_raw=...
op3v_regular_delta=...
op3v_regular_valid=1
```

Current boundary:

```text
OP3/VOPAMP3 is not part of the TIM1_CH4 injected ADC pair yet.
No sector switching has been implemented.
No phase-current reconstruction has been implemented.
No ia/ib/ic naming has been introduced.
```

Why this matters:

```text
The B-G431B-ESC1 current-sense model uses three OPAMPs with two ADCs.
OPAMP3/VOPAMP3 needs special handling before full three-phase current reconstruction.
This checkpoint proves the OPAMP3/VOPAMP3 internal route is alive without disturbing the working OP1/OP2 injected pair.
```

---

## Current status

Current active checkpoint:

```text
v0.5.24-warning-cleanup
```

The project currently has working split-module firmware that can:

```text
initialize STM32G431 clocks and peripherals directly
configure GPIO, analog pins, OPAMPs, ADCs, and TIM1 alternate functions
monitor VBUS and temperature through ADC
verify drive-pin and TIM1 safety state
command open-loop six-step PWM motor drive as a retained baseline
observe floating-phase BEMF during six-step operation
command open-loop sine/SPWM motor drive through the active sine path
run a quiet 96-step sine/SPWM open-loop bench test
perform TIM1_CH4-triggered injected ADC sampling for OP1/OP2
read OP3/VOPAMP3 through ADC2_IN18 as a regular diagnostic path
stop on button release
log status through RTT
build cleanly with no warnings
```

Current safety state from recent logs and tests:

```text
health_ok=1
af_ok=1
no_phase_overlap=1
timeout=0
runs stopped because of max_steps or button release, not a detected electrical fault
```

---

## What is still not confirmed

```text
calibrated motor phase current
exact OPAMP-to-motor-phase mapping
OP3/VOPAMP3 as a TIM1-triggered injected ADC sample
sector-switched two-ADC current reconstruction
ia/ib/ic naming
Clarke transform
Park transform
current PI loop
SVPWM
actual MOSFET gate waveform under PWM
actual high-side bootstrap voltage under continuous PWM
phase-node switching waveform on a scope
dead-time margin under higher current
closed-loop six-step commutation
sensorless zero-cross reliability
thermal behavior under sustained higher-power operation
```

A scope is still useful before aggressive PWM, higher current, closed-loop commutation, or FOC work.

---

## Next development directions

Near-term options:

```text
Commit/tag the current v0.5.24 warning-clean checkpoint.
Keep OP1/OP2 injected path unchanged as the stable timing proof.
Add an alternate diagnostic mode where ADC2 injected rank 1 samples OP3/VOPAMP3 instead of OP2.
Compare regular OP3/VOPAMP3 values against the alternate injected OP3/VOPAMP3 test.
Collect repeatability logs across several runs.
Keep nomenclature strict: injected vs regular, OP labels vs phase-current labels.
```

Longer-term FOC-oriented path:

```text
v0.5.24 clean OP3/VOPAMP3 regular diagnostic checkpoint
v0.5.x alternate injected OP3/VOPAMP3 diagnostic
v0.6.x current monitor calibration and phase mapping
v0.7.x structured motor-control timing, likely timer-driven update path
v0.8.x rotor-angle estimation experiments
v0.9.x first closed-loop current-control experiments
```

FOC-oriented modules later may include:

```text
foc.rs
clarke_park.rs
current.rs
svpwm.rs
observer.rs
motor_params.rs
```

Important limitation before FOC:

```text
FOC requires rotor angle estimation or sensing, usable calibrated current feedback, transforms, and current-loop control. The current sine/SPWM plus current-sense diagnostic checkpoint is a useful stepping stone, not a current-controlled system.
```

---

## Commit/tag convention

Use version tags matching tested firmware milestones, for example:

```text
v0.4.0-bemf-observe-openloop
v0.4.1-split-same-behavior
v0.5.2-openloop-sine-96-fast-loop
v0.5.23-op3v-regular-naming
v0.5.24-warning-cleanup
```
