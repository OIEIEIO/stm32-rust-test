# B-G431B-ESC1 Rust Bring-up

Rust bring-up project for the ST B-G431B-ESC1 Discovery kit using the STM32G431CB motor-control MCU.

Current confirmed milestone:

```text
v0.2.14-button-gated-u-twitch-prep
```

Major milestone:

```text
First motor-connected twitch test passed.
```

The motor was connected with no prop, powered from a current-limited bench supply at about 8 V, and each button press produced a small physical twitch/jump. The firmware returned to all-off after each pulse.

---

## Hardware

```text
Board: ST B-G431B-ESC1 Discovery kit
MCU: STM32G431CB
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

Confirmed alternate functions:

```text
PA8   AF6
PA9   AF6
PA10  AF6
PA12  AF6
PB15  AF4
PC13  AF4
```

---

## Bring-up summary

### GPIO / ADC / RTT

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
Potentiometer controls LED blink/log rate in earlier versions.
Temperature raw value rises when the ESC board is warmed by hand.
VBUS raw value rises when external ESC power input is raised.
```

---

## Drive-output bring-up milestones

### Safe low and TIM1 setup

Confirmed across early v0.2.x versions:

```text
All six drive inputs can be held low.
TIM1 can run internally.
TIM1 alternate-function routing works.
TIM1 CCER and MOE can be configured safely.
Forced-inactive all-low state works after complementary polarity correction.
```

Important finding:

```text
Forced inactive with normal complementary polarity did not make all six drive inputs low.
UH/VH/WH were low, but UL/VL/WL read high.
The corrected all-low forced-inactive state used inverted complementary output polarity.
```

Known good all-off forced-low state:

```text
expected_ccer=3549
UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

---

## Six individual command paths confirmed

Low-side command paths:

```text
v0.2.6-fix2  UL low-side passed
v0.2.7       VL low-side passed
v0.2.8       WL low-side passed
```

Representative confirmed values:

```text
UL only: tim1_ccer=5     UH=0 UL=1 VH=0 VL=0 WH=0 WL=0
VL only: tim1_ccer=80    UH=0 UL=0 VH=0 VL=1 WH=0 WL=0
WL only: tim1_ccer=1280  UH=0 UL=0 VH=0 VL=0 WH=0 WL=1
```

High-side input-command paths:

```text
v0.2.9   UH high-side input command passed
v0.2.10  VH high-side input command passed
v0.2.11  WH high-side input command passed
```

Representative confirmed values:

```text
UH only: tim1_ccer=1    UH=1 UL=0 VH=0 VL=0 WH=0 WL=0
VH only: tim1_ccer=16   UH=0 UL=0 VH=1 VL=0 WH=0 WL=0
WH only: tim1_ccer=256  UH=0 UL=0 VH=0 VL=0 WH=1 WL=0
```

Precise interpretation:

```text
Confirmed:
- TIM1 register setup is working.
- TIM1 alternate-function routing is working.
- Each intended gate-driver input can be commanded individually.
- Non-target drive inputs stay low during each single-output test.
- External VBUS caused no unexpected current draw during no-motor testing.
- Board stayed cool.

Not directly confirmed:
- actual MOSFET gate voltage
- actual MOSFET switching waveform
- bootstrap capacitor voltage
```

---

## Bootstrap-aware no-motor pulse tests

### v0.2.12 U-phase bootstrap-aware pulse

Confirmed no-motor U-phase sequence:

```text
all_off_before          UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
u_bootstrap_charge_ul   UH=0 UL=1 VH=0 VL=0 WH=0 WL=0
deadtime_after_ul       UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
u_highside_command_uh   UH=1 UL=0 VH=0 VL=0 WH=0 WL=0
all_off_cooldown        UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

Confirmed fields:

```text
state_ok=1
af_ok=1
pins_ok=1
no_u_overlap=1
tim1_ok=1
cycle_ok=1
```

### v0.2.13 all-phases bootstrap-aware pulse

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

External VBUS test:

```text
Motor disconnected.
Bench supply tested up to 12 V.
No unexpected current draw.
Board stayed cold.
VBUS ADC tracked the applied supply.
```

---

## First motor-connected twitch test

### v0.2.14 button-gated U->V twitch prep

Current confirmed version:

```text
v0.2.14-button-gated-u-twitch-prep
```

Firmware behavior:

```text
Default state: all outputs off.
Button press runs one short twitch sequence.
After twitch: all outputs off.
Waits for button release before allowing the next twitch.
```

Button press sequence:

```text
idle_all_off            UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
u_bootstrap_charge_ul   UH=0 UL=1 VH=0 VL=0 WH=0 WL=0
deadtime_after_ul       UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
twitch_drive_uh_vl      UH=1 UL=0 VH=0 VL=1 WH=0 WL=0
all_off_after_twitch    UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
wait_button_release     UH=0 UL=0 VH=0 VL=0 WH=0 WL=0
```

No-motor test confirmed:

```text
button trigger works
UL bootstrap-charge step works
all-off deadtime works
UH + VL phase-pair command works
no same-phase overlap
returns to all-off
twitch_summary cycle_ok=1
```

Motor-connected test conditions:

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
No continuous spin was attempted.
Board stayed cool.
No abnormal behavior reported.
```

Representative successful log fields:

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

Returned safely to all-off:

```text
state=all_off_after_twitch
state_ok=1
active_count=0

UH=0
UL=0
VH=0
VL=0
WH=0
WL=0
```

Summary field:

```text
twitch_summary cycle_ok=1 twitch_vector=UH_plus_VL
```

VBUS ADC behavior:

```text
VBUS raw increased with the bench supply.
Example during 8 V test: vbus_raw around 965, vbus_delta around 390.
```

Current-monitor behavior:

```text
Current-monitor raw values moved during/after the twitch, especially OP2.
This is consistent with a real motor-current event, but it is still raw ADC data and not calibrated current.
```

---

## Current status

```text
First motor-connected button-gated twitch succeeded.
Firmware returns to all-off after each twitch.
No prop was used.
No continuous commutation was attempted.
```

This is a major milestone. The project has moved from static command-path proof to real motor actuation.

---

## What is still not confirmed

```text
actual MOSFET gate voltage
actual high-side bootstrap voltage
phase-node switching waveform
calibrated motor current
dead-time margin under real PWM
continuous commutation stability
motor startup reliability
thermal behavior under repeated/longer drive
```

A scope or meter would be useful before aggressive PWM or longer spin tests.

---

## Recommended next step

Do not jump straight to continuous spin.

Recommended next step:

```text
v0.2.15 selectable twitch vectors
```

Purpose:

```text
Test all six valid six-step phase-pair vectors one at a time, button gated.
```

Valid vectors:

```text
UH + VL
UH + WL
VH + WL
VH + UL
WH + UL
WH + VL
```

Recommended behavior:

```text
Default all-off idle.
One button press advances to the next vector.
Each press performs:
  bootstrap charge for the target high-side phase
  all-off deadtime
  one short phase-pair twitch pulse
  all-off
  wait for button release
Log vector name, pins, TIM1 registers, VBUS, temp, and current-monitor raw values.
```

Test plan:

```text
1. Test v0.2.15 with no motor first.
2. Confirm each vector logs state_ok=1, pins_ok=1, no_phase_overlap=1.
3. Connect motor with no prop.
4. Use low voltage and strict current limit.
5. Button-test each vector one at a time.
6. Expect different tiny twitch directions/positions.
7. Stop if current jumps, board warms, motor locks hard, or logs show any failed state.
```

After selectable vector testing passes, the next later step can be very slow open-loop six-step commutation.

---

## Future path

Likely sequence:

```text
v0.2.15 selectable twitch vectors
v0.2.16 very slow open-loop six-step stepping, button gated
v0.2.17 low-duty timed stepping with adjustable delay
v0.2.18 cautious first slow spin attempt
```

Each stage should keep:

```text
no prop
low voltage
strict current limit
short test duration
automatic all-off fallback
clear RTT logs
```

---

## Commit/tag convention

Use version tags matching the tested firmware milestone, for example:

```text
v0.2.14-button-gated-u-twitch-prep
```

---

## Status summary

Current passed milestones:

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
```

---

Created: 2026-06-07
Updated milestone: v0.2.14-button-gated-u-twitch-prep
