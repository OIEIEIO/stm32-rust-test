# B-G431B-ESC1 Rust Bring-up

Rust bare-metal motor-control bring-up project for the ST B-G431B-ESC1 Discovery kit using the STM32G431CB motor-control MCU.

Current confirmed milestone:

```text
v0.5.2-openloop-sine-96-fast-loop
```

Current state:

```text
The source split is complete and confirmed successful.
The six-step implementation remains available as the known-good baseline.
The active runtime test path is now open-loop sine/SPWM.
The current sine/SPWM path uses a 96-step sine table and a fast CCR-only inner loop.
Heavy ADC/TIM1/drive health checks are no longer done every sine sample.
The motor spins smoothly and quietly in open-loop sine/SPWM bench testing.
No closed-loop commutation has been added yet.
No FOC/current loop has been added yet.
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
Firmware mode: open-loop sine/SPWM, 96-step table, fast inner loop
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

`cargo embed` is not required for this project workflow. In testing, `cargo run --release` has been the clean path.

Clean build artifacts:

```bash
cargo clean
cargo build --release
```

---

## Current project tree

The source split is complete and sine/SPWM has been added as a separate module:

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
    ├── drive.rs
    ├── gpio.rs
    ├── log.rs
    ├── main.rs
    ├── regs.rs
    ├── safety.rs
    ├── sine.rs
    ├── sixstep.rs
    └── tim1.rs
```

Module responsibilities:

```text
src/main.rs      startup sequence, top-level initialization, main control loop
src/regs.rs      register bases, register pointers, bit masks, board constants
src/gpio.rs      generic GPIO helpers, LED, button input
src/adc.rs       ADC setup, channel selection, raw ADC reads, board monitor snapshot
src/tim1.rs      TIM1 setup, PWM vectors, sine PWM helpers, CCER/CCMR/BDTR readback
src/drive.rs     DriveState, expected pin/CCER states, drive-pin readback, overlap checks
src/bemf.rs      BEMF pin setup, floating-phase selection, floating-phase ADC sampling
src/sixstep.rs   open-loop six-step ramp runner, retained as baseline
src/sine.rs      open-loop sine/SPWM runner, current active experiment
src/log.rs       structured RTT status logging
src/safety.rs    delay helpers and button-held delay/dead-man helper
```

Design principle:

```text
Keep control strategies separated:
sixstep.rs for six-step tests
sine.rs for open-loop sinusoidal/SPWM tests
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
PA2 / PA6 / PB1 op-amp or current-monitor raw ADC readings work.
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
GPIO alternate-function setup alone is not enough. TIM1 advanced-control behavior depends on CCER, CCMR, BDTR, MOE, polarity, and off-state handling. Every drive state needs readback checks.
```

---

### 5. Individual command paths

Low-side command paths were confirmed:

```text
UL low-side passed
VL low-side passed
WL low-side passed
```

Representative values:

```text
UL only: tim1_ccer=5     UH=0 UL=1 VH=0 VL=0 WH=0 WL=0
VL only: tim1_ccer=80    UH=0 UL=0 VH=0 VL=1 WH=0 WL=0
WL only: tim1_ccer=1280  UH=0 UL=0 VH=0 VL=0 WH=0 WL=1
```

High-side command paths were confirmed:

```text
UH high-side input command passed
VH high-side input command passed
WH high-side input command passed
```

Representative values:

```text
UH only: tim1_ccer=1    UH=1 UL=0 VH=0 VL=0 WH=0 WL=0
VH only: tim1_ccer=16   UH=0 UL=0 VH=1 VL=0 WH=0 WL=0
WH only: tim1_ccer=256  UH=0 UL=0 VH=0 VL=0 WH=1 WL=0
```

Confirmed from these tests:

```text
TIM1 register setup is working.
TIM1 alternate-function routing is working.
Each intended gate-driver input can be commanded individually.
Non-target drive inputs stay low during each single-output test.
External VBUS caused no unexpected current draw during no-motor testing.
Board stayed cool.
```

Not directly confirmed by these tests:

```text
actual MOSFET gate voltage
actual MOSFET switching waveform
actual bootstrap capacitor voltage
```

---

### 6. Bootstrap-aware pulse testing

Confirmed no-motor U-phase sequence:

```text
all_off_before          UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
u_bootstrap_charge_ul   UH=0 UL=1 VH=0 VL=0 WH=0 WL=0
deadtime_after_ul       UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
u_highside_command_uh   UH=1 UL=0 VH=0 VL=0 WH=0 WL=0
all_off_cooldown        UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

Confirmed no-motor U/V/W sequence:

```text
UL charge -> all off -> UH command -> all off
VL charge -> all off -> VH command -> all off
WL charge -> all off -> WH command -> all off
```

Confirmed fields:

```text
state_ok=1
af_ok=1
pins_ok=1
no_phase_overlap=1
active_count_ok=1
tim1_ok=1
cycle_ok=1
```

Bring-up lesson:

```text
The high-side path needs bootstrap awareness. Early tests treated bootstrap precharge as an explicit step before moving into PWM-based operation.
```

---

### 7. First motor-connected twitch

The first motor-connected twitch test passed.

Test conditions:

```text
Small 2212 motor
No prop
Bench supply around 8 V
Current-limited supply
Button-gated single pulse
No continuous commutation
```

Observed result:

```text
Each button press produced a small physical twitch/jump.
The firmware returned to all-off after each pulse.
Board stayed cool.
No abnormal behavior was reported.
```

Representative successful vector:

```text
state=twitch_drive_uh_vl
state_ok=1
pins_ok=1
no_phase_overlap=1
active_count=2
active_count_ok=1
tim1_ok=1
expected_ccer=81
tim1_ccer=81

UH=1
UL=0
VH=0
VL=1
WH=0
WL=0
```

Bring-up lesson:

```text
The static command-path tests translated into real motor actuation. This was the transition from register and pin proof to real electromechanical behavior.
```

---

### 8. Open-loop six-step spin baseline

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

Safety fields observed:

```text
health_ok=1
button=1
af_ok=1
no_phase_overlap=1
timeout=0
```

After-run idle state observed:

```text
state=idle_all_off
UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
active_count=0
active_count_ok=1
tim1_ok=1
forced_modes_ok=1
```

PWM carrier:

```text
TIM1 ARR = 799
Default HSI16 timer clock assumption gives about 20 kHz PWM carrier.
The audible buzz heard during six-step tests is more likely torque ripple, open-loop slip/catch behavior, logging/timing disturbance, or bench-supply current limiting than the 20 kHz PWM carrier itself.
```

Bring-up lesson:

```text
Six-step is useful for first motion because one phase naturally floats and can be sampled for BEMF. It is also mechanically rough, especially during open-loop startup and low-speed operation.
```

---

### 9. BEMF observation

Current BEMF mode:

```text
Observe only
No closed-loop commutation
Floating phase selected from the six-step vector
Four ADC samples logged as b0..b3
```

Current log fields are intentionally concrete:

```text
vector=...
float_phase=...
b0=...
b1=...
b2=...
b3=...
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

### 10. Source split validation

Split milestone:

```text
v0.4.1-split-same-behavior
```

Build result:

```text
cargo build --release passed.
The previous DeadtimeAllOff dead-code warning was cleaned up by removing the unused placeholder state.
The build was warning-free after that cleanup.
```

No-motor runtime check:

```text
cargo run --release flashed and ran successfully.
Startup log appeared normally.
Idle all-off state was confirmed.
ADC logs remained present.
BEMF startup text remained present.
```

Motor/no-prop runtime check:

```text
The motor test behavior matched the pre-split behavior.
The run completed the programmed vector count.
The firmware returned to idle all-off after the run.
The logs showed the expected safety fields.
```

Conclusion:

```text
The source split is confirmed successful.
The v0.4.1 baseline remains useful as the known-good split six-step state.
```

---

### 11. Open-loop sine/SPWM milestone

Current milestone:

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
The first coarse 24-step sine table worked but still had audible buzz.
Changing PWM carrier from about 20 kHz to about 32 kHz did not significantly reduce the noise.
The 96-step sine table made the motor much quieter and smoother, but the first version was very slow.
The v0.5.2 fast-loop rewrite moved heavy ADC/TIM1/drive health checks out of every sine step.
The fast inner loop now updates CCR1/CCR2/CCR3 directly, checks the button, and delays.
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
The major improvement came from separating fast waveform generation from slow diagnostics. The 96-step sine table made the waveform quiet, but the per-step health/readback loop dominated timing until v0.5.2 moved diagnostics out of the fast path.
```

---

## Current status

Current active milestone:

```text
v0.5.2-openloop-sine-96-fast-loop
```

The project currently has working split-module firmware that can:

```text
initialize STM32G431 clocks and peripherals directly
configure GPIO, analog pins, and TIM1 alternate functions
monitor VBUS and temperature through ADC
verify drive-pin and TIM1 safety state
command open-loop six-step PWM motor drive as a retained baseline
observe floating-phase BEMF during six-step operation
command open-loop sine/SPWM motor drive through the active sine path
run a quiet 96-step sine/SPWM open-loop bench test
stop on button release
log status through RTT
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
actual MOSFET gate waveform under PWM
actual high-side bootstrap voltage under continuous PWM
phase-node switching waveform on a scope
dead-time margin under higher current
closed-loop six-step commutation
sensorless zero-cross reliability
FOC current-loop behavior
thermal behavior under sustained higher-power operation
```

A scope is still useful before aggressive PWM, higher current, closed-loop commutation, or FOC work.

---

## Next development directions

Near-term options:

```text
Commit/tag the current v0.5.2 sine/SPWM milestone.
Add RPM/timing estimate fields to the sine logs.
Add potentiometer-controlled speed or amplitude target.
Tune sine ramp parameters after the fast-loop improvement.
Keep sixstep.rs available as the BEMF-observe baseline.
```

Longer-term FOC-oriented path:

```text
v0.5.2 open-loop sine/SPWM fast loop
v0.5.x sine timing/ramp tuning and safety cleanup
v0.6.x current monitor calibration
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
FOC requires rotor angle estimation or sensing, usable current feedback, transforms, and current-loop control. The current sine/SPWM milestone is a useful stepping stone, not a current-controlled system.
```

---

## Commit/tag convention

Use version tags matching tested firmware milestones, for example:

```text
v0.4.0-bemf-observe-openloop
v0.4.1-split-same-behavior
v0.5.2-openloop-sine-96-fast-loop
```
