# B-G431B-ESC1 Rust Bring-up

Rust bring-up project for the ST B-G431B-ESC1 Discovery kit using the STM32G431CB motor-control MCU.

Current confirmed milestone:

```text
v0.2.8-single-low-side-wl-on
```

This project is focused on safe, step-by-step board bring-up before attempting to spin a BLDC motor.

---

## Hardware

Board:

```text
ST B-G431B-ESC1 Discovery kit
STM32G431CB MCU
L6387 gate drivers
STL180N6F7 MOSFETs
```

Development/debug:

```text
ST-LINK/V2-1
probe-rs / cargo-embed
RTT logging
Ubuntu host
```

Current target:

```text
thumbv7em-none-eabihf
```

Confirmed probe:

```text
STM32G431CB
chipid: 0x468
flash: 128 KiB
sram: 32 KiB
```

---

## Useful commands

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

---

## Confirmed board signals

From schematic and bring-up testing:

```text
PC6   STATUS LED
PC10  user button input
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

Confirmed alternate functions used:

```text
PA8   AF6
PA9   AF6
PA10  AF6
PA12  AF6
PB15  AF4
PC13  AF4
```

---

## Bring-up history

### v0.1.x GPIO / ADC / RTT

Confirmed:

```text
PC6 LED output works.
PC10 button input works.
PB12 potentiometer ADC works.
PB14 temperature feedback ADC works.
PA0 VBUS feedback ADC works.
PA2/PA6/PB1 op-amp/current monitor raw ADC readings work.
RTT logging works through probe-rs.
```

Observed behavior:

```text
Potentiometer controls LED blink rate.
Temperature raw value rises when the ESC board is warmed by hand.
VBUS raw value rises when external ESC power input is raised.
```

---

### v0.2.0 drive pins safe-low

Confirmed all six L6387 input pins can be configured as GPIO outputs and held low:

```text
UH=0
UL=0
VH=0
VL=0
WH=0
WL=0
drive_safe=1
```

No motor connected.

---

### v0.2.1 TIM1 internal counter, MOE off

Confirmed TIM1 can be configured and run internally while drive pins remain GPIO LOW.

Expected/confirmed fields:

```text
tim1_counting=1
tim1_arr=3999
tim1_ccr1=0
tim1_ccr2=0
tim1_ccr3=0
tim1_ccer=0
tim1_moe=0
drive_safe=1
```

No motor connected.

---

### v0.2.2 TIM1 alternate-function pins, MOE off

Confirmed the six drive pins can be moved to TIM1 alternate-function mode while TIM1 outputs remain disabled.

Expected/confirmed fields:

```text
af_ok=1
tim1_counting=1
tim1_ccer=0
tim1_moe=0
UH_af=6
VH_af=6
WH_af=6
VL_af=6
WL_af=4
UL_af=4
```

No motor connected.

---

### v0.2.3-fix1 TIM1 CCER enabled, MOE off

Confirmed TIM1 CC output enable bits can be configured while the main output enable remains off.

Expected/confirmed fields:

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
```

No motor connected.

---

### v0.2.4 TIM1 MOE on, CCER off

Confirmed TIM1 main output enable can be set while CCER remains disabled.

Expected/confirmed fields:

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
```

No motor connected.

---

### v0.2.5 first attempt: forced inactive, CCER on, MOE on

Important finding:

```text
Forced inactive with normal complementary polarity did not make all six drive inputs low.
UH/VH/WH were low, but UL/VL/WL read high.
```

This version was not treated as a passed milestone.

---

### v0.2.5-fix1 forced inactive all-low

Confirmed version:

```text
v0.2.5-fix1-tim1-forced-inactive-all-low
```

This version:

```text
TIM1 pins are alternate function.
TIM1 counter is running.
CCER is enabled.
MOE is enabled.
Channels are forced inactive.
Complementary output polarity is inverted so all six drive inputs read low.
CCR1/2/3 remain zero.
No PWM duty is applied.
No intentional gate switching is commanded.
```

Expected/confirmed fields:

```text
startup_overall_safe=1
overall_safe=1
tim1_register_ok=1
af_ok=1
pins_low=1
tim1_counting=1
tim1_ccer=3549
tim1_ccer_expected=1
tim1_moe=1
tim1_ossi=1
tim1_ossr=1
forced_inactive_ok=1

UH_pin=0
UL_pin=0
VH_pin=0
VL_pin=0
WH_pin=0
WL_pin=0
```

External VBUS no-motor test:

```text
Bench supply was applied up to about 10 V.
Motor disconnected.
Board stayed cold.
Supply current appeared negligible.
VBUS ADC responded correctly and returned to USB baseline when supply was lowered/removed.
```

---

## Low-side command-path tests

These tests deliberately commanded one low-side driver input high at a time.

Important limitation:

```text
Confirmed: STM32/TIM1 to gate-driver input command path.
Not directly confirmed: actual MOSFET gate voltage or switching waveform.
```

No motor was connected for any of these tests.

---

### v0.2.6-fix2 UL low-side only

Confirmed:

```text
ul_test_ok=1
tim1_register_ok=1
af_ok=1
ul_only_active=1
tim1_counting=1
tim1_ccer=5
tim1_ccer_expected=1
tim1_moe=1
forced_modes_ok=1

UH_pin=0
UL_pin=1
VH_pin=0
VL_pin=0
WH_pin=0
WL_pin=0
```

External VBUS test:

```text
Motor disconnected.
Bench supply applied.
No meaningful supply current draw.
Board stayed cold.
```

---

### v0.2.7 VL low-side only

Confirmed:

```text
vl_test_ok=1
tim1_register_ok=1
af_ok=1
vl_only_active=1
tim1_counting=1
tim1_ccer=80
tim1_ccer_expected=1
tim1_moe=1
forced_modes_ok=1

UH_pin=0
UL_pin=0
VH_pin=0
VL_pin=1
WH_pin=0
WL_pin=0
```

External VBUS test:

```text
Motor disconnected.
Bench supply applied.
No meaningful supply current draw.
Board stayed cold.
```

---

### v0.2.8 WL low-side only

Current confirmed version:

```text
v0.2.8-single-low-side-wl-on
```

Confirmed:

```text
wl_test_ok=1
tim1_register_ok=1
af_ok=1
wl_only_active=1
tim1_counting=1
tim1_ccer=1280
tim1_ccer_expected=1
tim1_moe=1
forced_modes_ok=1

UH_pin=0
UL_pin=0
VH_pin=0
VL_pin=0
WH_pin=0
WL_pin=1
```

External VBUS test:

```text
Motor disconnected.
Bench supply tested up to 12 V.
No bench supply load observed.
Board stayed cold to the touch.
```

---

## Current confirmed milestone

```text
UL low-side command path works.
VL low-side command path works.
WL low-side command path works.
External VBUS tested up to 12 V with motor disconnected.
No bench supply load observed.
Board stayed cold.
No unexpected drive pin state observed.
```

This is a good milestone before moving toward high-side/bootstrap-aware testing or motor-connected testing.

---

## Current safety rule

Do not connect a motor for the current firmware versions.

The current low-side tests only validate one low-side command path at a time:

```text
One low-side driver input active.
All high-side inputs off.
Other low-side inputs off.
No PWM.
No motor.
No load path expected.
```

---

## Next likely step

Next step should be discussed before coding.

Recommended next technical direction:

```text
High-side path preparation / bootstrap-aware testing.
Still no motor at first.
Use current-limited bench supply.
Avoid simultaneous high-side and low-side conduction.
Review L6387 bootstrap behavior before commanding high-side outputs.
```

A first motor spin test comes later.

---

## Commit/tag convention

Use version tags matching the tested firmware milestone, for example:

```text
v0.2.8-single-low-side-wl-on
```

---

## Status

Current status:

```text
v0.2.6-fix2 UL low-side passed.
v0.2.7 VL low-side passed.
v0.2.8 WL low-side passed.
External VBUS tested up to 12 V with no motor and no load.
Ready to commit the three-low-side phase-test milestone.
```
