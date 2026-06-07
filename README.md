# B-G431B-ESC1 Rust Bring-up

Rust bring-up project for the ST B-G431B-ESC1 Discovery kit using the STM32G431CB motor-control MCU.

Current confirmed milestone:

```text
v0.2.11-single-high-side-wh-command
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
Potentiometer controls LED blink/log rate.
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

## High-side input-command tests

These tests deliberately commanded one high-side gate-driver input high at a time.

Important limitation:

```text
Confirmed: STM32/TIM1 to gate-driver input command path.
Not directly confirmed: actual high-side MOSFET gate voltage, bootstrap voltage, or switching waveform.
```

No motor was connected for any of these tests.

The high-side tests are useful because they verify the command path and pin mapping without intentionally creating a VBUS-to-ground current path.

---

### v0.2.9 UH high-side command only

Confirmed:

```text
uh_test_ok=1
tim1_register_ok=1
af_ok=1
uh_only_active=1
tim1_counting=1
tim1_ccer=1
tim1_ccer_expected=1
tim1_moe=1
forced_modes_ok=1

UH_pin=1
UL_pin=0
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
Board stayed cool.
```

---

### v0.2.10 VH high-side command only

Confirmed:

```text
vh_test_ok=1
tim1_register_ok=1
af_ok=1
vh_only_active=1
tim1_counting=1
tim1_ccer=16
tim1_ccer_expected=1
tim1_moe=1
forced_modes_ok=1

UH_pin=0
UL_pin=0
VH_pin=1
VL_pin=0
WH_pin=0
WL_pin=0
```

External VBUS test:

```text
Motor disconnected.
Bench supply applied.
No meaningful supply current draw.
Board stayed cool.
```

---

### v0.2.11 WH high-side command only

Current confirmed version:

```text
v0.2.11-single-high-side-wh-command
```

Confirmed:

```text
wh_test_ok=1
tim1_register_ok=1
af_ok=1
wh_only_active=1
tim1_counting=1
tim1_ccer=256
tim1_ccer_expected=1
tim1_moe=1
forced_modes_ok=1

UH_pin=0
UL_pin=0
VH_pin=0
VL_pin=0
WH_pin=1
WL_pin=0
```

External VBUS test:

```text
Motor disconnected.
Bench supply applied.
No meaningful supply current draw was expected.
No unexpected pin state was observed.
Board stayed cool.
```

---

## Current confirmed milestone

All six individual command paths have now been tested one at a time:

```text
UL low-side command path works.
VL low-side command path works.
WL low-side command path works.

UH high-side input command path works.
VH high-side input command path works.
WH high-side input command path works.
```

More precise interpretation:

```text
Confirmed:
- TIM1 register setup is working.
- TIM1 alternate-function routing is working.
- Each intended gate-driver input can be commanded individually.
- Non-target drive inputs stay low during each single-output test.
- External VBUS testing caused no unexpected current draw.
- Board stayed cool during these no-motor tests.

Not yet directly confirmed:
- Actual MOSFET gate voltage.
- Actual MOSFET drain/source conduction.
- Bootstrap capacitor behavior.
- Real switching waveform.
- Motor phase output behavior under load.
```

---

## Why outputs are tested one at a time

The one-at-a-time method reduces risk and helps catch errors before motor-connected testing.

It helps verify:

```text
wrong pin mapping
wrong alternate-function selection
wrong TIM1 channel selection
wrong polarity
unexpected complementary output
unexpected two-FET activation in one phase
shoot-through risk
unexpected current draw under VBUS
heat or bad board behavior
```

With only one driver input active and no motor connected, there is no intended current path from VBUS to ground.

---

## Current safety rule

Do not connect a motor for the current firmware versions.

Current tests only validate one command path at a time:

```text
One driver input active.
All other driver inputs inactive.
No PWM duty.
No commutation.
No motor.
No intentional load path.
```

---

## Next research topic

Before writing the next firmware step, research and review the L6387 high-side bootstrap behavior.

Specific items to research:

```text
L6387 bootstrap capacitor charging path.
Minimum low-side on-time needed to charge bootstrap.
Bootstrap diode behavior.
High-side UVLO behavior.
How long the high-side can stay on from bootstrap charge.
Recommended bootstrap capacitor sizing.
Dead-time requirements.
Safe startup state.
Safe pulse sequencing.
How to avoid high-side/low-side shoot-through.
```

Board-specific schematic items to review:

```text
bootstrap capacitors on each L6387 driver
bootstrap diodes
gate resistors
phase output nodes
shunt/current monitor path
VBUS divider
current feedback op-amp outputs
```

---

## Next likely firmware step

Recommended next firmware step:

```text
v0.2.12 bootstrap-aware single-phase pulse prep
```

Likely test concept, still motor disconnected:

```text
1. Start all outputs safe/inactive.
2. Briefly command the matching low-side input to create a bootstrap charge opportunity.
3. Turn all outputs off.
4. Briefly command the matching high-side input.
5. Return all outputs off.
6. Repeat slowly.
7. Keep detailed logs of requested state, register state, pin readback, VBUS, temperature, and current-monitor raw ADC values.
```

Initial target phase:

```text
U phase:
UL bootstrap-charge pulse
all off
UH command pulse
all off
repeat slowly
```

Safety constraints for this next step:

```text
No motor.
Current-limited bench supply.
Start around 5 V.
Low current limit.
Stop immediately if current rises unexpectedly.
Stop immediately if anything warms up.
Avoid simultaneous UH and UL.
Avoid enabling any other phase.
```

The goal of the next firmware is still not motor spin. The goal is to prepare a controlled bootstrap-aware pulse pattern and watch for safe board behavior.

---

## Future steps after bootstrap-aware testing

Possible later sequence:

```text
1. Bootstrap-aware U phase pulse test, no motor.
2. Repeat for V phase, no motor.
3. Repeat for W phase, no motor.
4. Review whether a scope or meter is available for gate/phase verification.
5. Create very low-duty open-loop six-step commutation firmware.
6. Connect small BLDC motor with no prop.
7. Use low VBUS and strict current limit.
8. Attempt first brief motor twitch.
9. Only later attempt slow controlled spin.
```

---

## Commit/tag convention

Use version tags matching the tested firmware milestone, for example:

```text
v0.2.11-single-high-side-wh-command
```

---

## Status

Current status:

```text
v0.2.6-fix2 UL low-side passed.
v0.2.7 VL low-side passed.
v0.2.8 WL low-side passed.

v0.2.9 UH high-side input-command passed.
v0.2.10 VH high-side input-command passed.
v0.2.11 WH high-side input-command passed.

Ready to commit the six individual command-path milestone.
Next: research L6387 bootstrap behavior before writing bootstrap-aware pulse firmware.
```

---

Created: 2026-06-07
Updated milestone: v0.2.11-single-high-side-wh-command
