// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.4.0-bemf-observe-openloop
// Purpose: STM32G431CB Rust: PWM open-loop six-step with BEMF sense
//          INSTRUMENTATION (observe only; loop stays open)
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
//
// Change summary vs v0.3.2:
//   - DRIVE UNCHANGED: same ramp, alignment, dead-man, timer-based
//     commutation. The loop is still OPEN. No commutation decision
//     uses BEMF. The motor spins exactly as v0.3.x did.
//   - Added BEMF sense instrumentation so the floating-phase back-EMF
//     can be OBSERVED before closing the loop:
//       * GPIO_BEMF = PB5 driven as INPUT / high-Z -> divider DISABLED
//         -> PWM-OFF, ground-referenced sampling (correct for low duty).
//       * BEMF1=PA4 (U/OUT1, ADC2_IN17), BEMF2=PC4 (V/OUT2, ADC2_IN5),
//         BEMF3=PB11 (W/OUT3, ADC2_IN14). ADC2 is the BEMF engine.
//       * Each commutation step: blank (demag), then sample the
//         FLOATING phase 4x across the step inside the PWM-OFF window
//         (CNT polled against the duty), logged as b0..b3.
//   - Run log is now per-step "bemf" lines (was per-erev heartbeat),
//     so one electrical rev of log shows the BEMF trajectory.
//   - Speed lowered for legible logging: RAMP_MIN_VECTOR_DELAY
//     50000 -> 150000 (~150 RPM). BEMF smaller than at 450 RPM but
//     clearly present. MAX_VECTOR_STEPS_PER_HOLD 1200 -> 600.
//   - Duty UNCHANGED (20% max).
//   - VERIFY FIRST: confirm each BEMF channel responds to its own
//     phase (sanity-check the PA4/PC4/PB11 -> ADC2 channel numbers
//     against DS12589) before trusting the waveform.
//   - NOTE: still no closed loop; health_ok cannot detect a slip.
//
// Change summary v0.3.1 vs v0.3.0:
//   - PWM frequency raised to ~20 kHz (inaudible). ARR 3999 -> 799 on
//     the default HSI16 clock (16 MHz / 800 = 20 kHz).
//   - Duty constants rescaled to the new ARR to preserve the SAME
//     effective voltages as the v0.3.0 run (10% align/start, 20% max).
//
// Change summary of v0.3.0 vs v0.2.16 (forced-active baseline):
//   - High-side drive: FORCED_ACTIVE (100% on) -> PWM_MODE_1 with duty.
//   - Commutation: direct vector->vector, no per-step bootstrap /
//     deadtime / all-off windows (PWM off-time freewheel recharges
//     the high-side bootstrap caps via the low-side body diode).
//   - Bootstrap precharge: done ONCE at run start (reuses the
//     confirmed Bootstrap*L states), not per commutation.
//   - Alignment: rotor is parked on one vector before the ramp.
//   - Logging: split. Heavy verification dump kept for startup and
//     fault only; a compact heartbeat is emitted once per electrical
//     rev during the run.
//   - Ramp constants retuned for the PWM regime (PWM bounds current,
//     so the start can be faster than the forced-active version).
//
// Unchanged / confirmed-working and reused as-is:
//   GPIO base setup, TIM1 base setup, drive-pin AF mapping, ADC setup,
//   the commutation CCER table (expected_ccer), read_drive,
//   pins_match_state, no_phase_overlap, dead-man release path,
//   ramp_delay_for_step, startup readback, FaultAllOff handling.
//
// PWM frequency note: ARR is 799 on the default HSI16 clock, giving
// ~20 kHz (above hearing). If a PLL has been configured elsewhere the
// frequency scales with the timer clock; recompute ARR if so.
// ================================================================

#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};

use cortex_m::asm;
use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

// ------------------------------------------------------------
// STM32G4 peripheral base addresses
// ------------------------------------------------------------

const RCC_BASE: usize = 0x4002_1000;
const GPIOA_BASE: usize = 0x4800_0000;
const GPIOB_BASE: usize = 0x4800_0400;
const GPIOC_BASE: usize = 0x4800_0800;

const ADC1_BASE: usize = 0x5000_0000;
const ADC2_BASE: usize = 0x5000_0100;
const ADC12_COMMON_BASE: usize = 0x5000_0300;

const TIM1_BASE: usize = 0x4001_2C00;

// ------------------------------------------------------------
// RCC registers
// ------------------------------------------------------------

const RCC_AHB2ENR: *mut u32 = (RCC_BASE + 0x4C) as *mut u32;
const RCC_APB2ENR: *mut u32 = (RCC_BASE + 0x60) as *mut u32;

const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;
const RCC_AHB2ENR_ADC12EN: u32 = 1 << 13;

const RCC_APB2ENR_TIM1EN: u32 = 1 << 11;

// ------------------------------------------------------------
// GPIO registers
// ------------------------------------------------------------

const GPIOA_MODER: *mut u32 = (GPIOA_BASE + 0x00) as *mut u32;
const GPIOA_OTYPER: *mut u32 = (GPIOA_BASE + 0x04) as *mut u32;
const GPIOA_OSPEEDR: *mut u32 = (GPIOA_BASE + 0x08) as *mut u32;
const GPIOA_PUPDR: *mut u32 = (GPIOA_BASE + 0x0C) as *mut u32;
const GPIOA_IDR: *const u32 = (GPIOA_BASE + 0x10) as *const u32;
const GPIOA_BSRR: *mut u32 = (GPIOA_BASE + 0x18) as *mut u32;
const GPIOA_AFRL: *mut u32 = (GPIOA_BASE + 0x20) as *mut u32;
const GPIOA_AFRH: *mut u32 = (GPIOA_BASE + 0x24) as *mut u32;
const GPIOA_ASCR: *mut u32 = (GPIOA_BASE + 0x2C) as *mut u32;

const GPIOB_MODER: *mut u32 = (GPIOB_BASE + 0x00) as *mut u32;
const GPIOB_OTYPER: *mut u32 = (GPIOB_BASE + 0x04) as *mut u32;
const GPIOB_OSPEEDR: *mut u32 = (GPIOB_BASE + 0x08) as *mut u32;
const GPIOB_PUPDR: *mut u32 = (GPIOB_BASE + 0x0C) as *mut u32;
const GPIOB_IDR: *const u32 = (GPIOB_BASE + 0x10) as *const u32;
const GPIOB_BSRR: *mut u32 = (GPIOB_BASE + 0x18) as *mut u32;
const GPIOB_AFRL: *mut u32 = (GPIOB_BASE + 0x20) as *mut u32;
const GPIOB_AFRH: *mut u32 = (GPIOB_BASE + 0x24) as *mut u32;
const GPIOB_ASCR: *mut u32 = (GPIOB_BASE + 0x2C) as *mut u32;

const GPIOC_MODER: *mut u32 = (GPIOC_BASE + 0x00) as *mut u32;
const GPIOC_OTYPER: *mut u32 = (GPIOC_BASE + 0x04) as *mut u32;
const GPIOC_OSPEEDR: *mut u32 = (GPIOC_BASE + 0x08) as *mut u32;
const GPIOC_PUPDR: *mut u32 = (GPIOC_BASE + 0x0C) as *mut u32;
const GPIOC_IDR: *const u32 = (GPIOC_BASE + 0x10) as *const u32;
const GPIOC_BSRR: *mut u32 = (GPIOC_BASE + 0x18) as *mut u32;
const GPIOC_AFRL: *mut u32 = (GPIOC_BASE + 0x20) as *mut u32;
const GPIOC_AFRH: *mut u32 = (GPIOC_BASE + 0x24) as *mut u32;
const GPIOC_ASCR: *mut u32 = (GPIOC_BASE + 0x2C) as *mut u32;

// ------------------------------------------------------------
// TIM1 registers
// ------------------------------------------------------------

const TIM1_CR1: *mut u32 = (TIM1_BASE + 0x00) as *mut u32;
const TIM1_CR2: *mut u32 = (TIM1_BASE + 0x04) as *mut u32;
const TIM1_EGR: *mut u32 = (TIM1_BASE + 0x14) as *mut u32;
const TIM1_CCMR1: *mut u32 = (TIM1_BASE + 0x18) as *mut u32;
const TIM1_CCMR2: *mut u32 = (TIM1_BASE + 0x1C) as *mut u32;
const TIM1_CCER: *mut u32 = (TIM1_BASE + 0x20) as *mut u32;
const TIM1_CNT: *mut u32 = (TIM1_BASE + 0x24) as *mut u32;
const TIM1_PSC: *mut u32 = (TIM1_BASE + 0x28) as *mut u32;
const TIM1_ARR: *mut u32 = (TIM1_BASE + 0x2C) as *mut u32;
const TIM1_RCR: *mut u32 = (TIM1_BASE + 0x30) as *mut u32;
const TIM1_CCR1: *mut u32 = (TIM1_BASE + 0x34) as *mut u32;
const TIM1_CCR2: *mut u32 = (TIM1_BASE + 0x38) as *mut u32;
const TIM1_CCR3: *mut u32 = (TIM1_BASE + 0x3C) as *mut u32;
const TIM1_BDTR: *mut u32 = (TIM1_BASE + 0x44) as *mut u32;

const TIM1_CR1_CEN: u32 = 1 << 0;
const TIM1_CR1_ARPE: u32 = 1 << 7;
const TIM1_EGR_UG: u32 = 1 << 0;

const TIM1_BDTR_OSSI: u32 = 1 << 10;
const TIM1_BDTR_OSSR: u32 = 1 << 11;
const TIM1_BDTR_MOE: u32 = 1 << 15;

const TIM1_CCMR_FORCED_INACTIVE: u32 = 0b100;
const TIM1_CCMR_FORCED_ACTIVE: u32 = 0b101;
const TIM1_CCMR_PWM_MODE_1: u32 = 0b110;

// ------------------------------------------------------------
// CCER command states (UNCHANGED commutation table)
// ------------------------------------------------------------

const TIM1_CCER_ALL_OFF_FORCED_LOW: u32 =
    (1 << 0) | (1 << 2) | (1 << 3) |
    (1 << 4) | (1 << 6) | (1 << 7) |
    (1 << 8) | (1 << 10) | (1 << 11);

const TIM1_CCER_BOOTSTRAP_U_LOW: u32 = (1 << 0) | (1 << 2);
const TIM1_CCER_BOOTSTRAP_V_LOW: u32 = (1 << 4) | (1 << 6);
const TIM1_CCER_BOOTSTRAP_W_LOW: u32 = (1 << 8) | (1 << 10);

const TIM1_CCER_VECTOR_UH_VL: u32 = (1 << 0) | (1 << 4) | (1 << 6);
const TIM1_CCER_VECTOR_UH_WL: u32 = (1 << 0) | (1 << 8) | (1 << 10);
const TIM1_CCER_VECTOR_VH_WL: u32 = (1 << 4) | (1 << 8) | (1 << 10);
const TIM1_CCER_VECTOR_VH_UL: u32 = (1 << 4) | (1 << 0) | (1 << 2);
const TIM1_CCER_VECTOR_WH_UL: u32 = (1 << 8) | (1 << 0) | (1 << 2);
const TIM1_CCER_VECTOR_WH_VL: u32 = (1 << 8) | (1 << 4) | (1 << 6);

const TIM1_TEST_PSC: u32 = 0;
const TIM1_TEST_ARR: u32 = 799;

// ------------------------------------------------------------
// Idle / housekeeping timing
// ------------------------------------------------------------

const IDLE_LOG_DELAY: u32 = 5_000_000;
const IDLE_LED_ON_DELAY: u32 = 250_000;
const IDLE_LED_OFF_DELAY: u32 = 2_500_000;

const STATE_SETTLE_DELAY: u32 = 150_000;
const BOOTSTRAP_HOLD_DELAY: u32 = 400_000;
const RELEASE_CHECK_CHUNK: u32 = 40_000;

// ------------------------------------------------------------
// PWM open-loop drive constants
// ------------------------------------------------------------
// Duty is expressed directly as a CCR count against ARR (799).
// 80/800 = 10%, 120/800 = 15%, 160/800 = 20%.

const PWM_DUTY_ALIGN: u32 = 80;
const PWM_DUTY_RUN_START: u32 = 80;
const PWM_DUTY_RUN_MAX: u32 = 100;
const PWM_DUTY_INC_PER_EREV: u32 = 5;

const ALIGN_HOLD_DELAY: u32 = 1_500_000;

// Ramp for the BEMF-observe build: lower top speed than the ceiling
// test so per-step logging is legible and one electrical rev of log
// shows the BEMF trajectory across all three phases.
// ~150 RPM target: 150000 cyc/vec -> 900000 cyc/erev -> ~17.8 Hz elec
// -> /7 pole-pairs -> ~2.5 rev/s -> ~152 RPM on a 2212.
const RAMP_START_VECTOR_DELAY: u32 = 500_000;
const RAMP_MIN_VECTOR_DELAY: u32 = 1_000;
const RAMP_DECREMENT_PER_ELECTRICAL_REV: u32 = 100_000;
const MAX_VECTOR_STEPS_PER_HOLD: u32 = 1000;

const TEMP_DELTA_ABORT_RAW: i32 = 500;

// ------------------------------------------------------------
// ADC register helpers
// ------------------------------------------------------------

fn adc_isr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x00) as *mut u32
}

fn adc_cr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x08) as *mut u32
}

fn adc_cfgr(adc_base: usize) -> *mut u32 {
    (adc_base + 0x0C) as *mut u32
}

fn adc_smpr1(adc_base: usize) -> *mut u32 {
    (adc_base + 0x14) as *mut u32
}

fn adc_smpr2(adc_base: usize) -> *mut u32 {
    (adc_base + 0x18) as *mut u32
}

fn adc_sqr1(adc_base: usize) -> *mut u32 {
    (adc_base + 0x30) as *mut u32
}

fn adc_dr(adc_base: usize) -> *const u32 {
    (adc_base + 0x40) as *const u32
}

fn adc_difsel(adc_base: usize) -> *mut u32 {
    (adc_base + 0xB0) as *mut u32
}

const ADC12_CCR: *mut u32 = (ADC12_COMMON_BASE + 0x08) as *mut u32;

const ADC_ISR_ADRDY: u32 = 1 << 0;
const ADC_ISR_EOC: u32 = 1 << 2;
const ADC_ISR_EOS: u32 = 1 << 3;

const ADC_CR_ADEN: u32 = 1 << 0;
const ADC_CR_ADSTART: u32 = 1 << 2;
const ADC_CR_ADVREGEN: u32 = 1 << 28;
const ADC_CR_DEEPPWD: u32 = 1 << 29;
const ADC_CR_ADCAL: u32 = 1 << 31;

// ------------------------------------------------------------
// Board pins
// ------------------------------------------------------------

const STATUS_LED_PIN: u32 = 6;
const USER_BUTTON_PIN: u32 = 10;

const VBUS_PIN: u32 = 0;
const OP1_OUT_PIN: u32 = 2;
const OP2_OUT_PIN: u32 = 6;

const OP3_OUT_PIN: u32 = 1;
const POT_PIN: u32 = 12;
const TEMP_PIN: u32 = 14;

const DRIVE_UH_PIN: u32 = 8;
const DRIVE_VH_PIN: u32 = 9;
const DRIVE_WH_PIN: u32 = 10;
const DRIVE_VL_PIN: u32 = 12;
const DRIVE_WL_PIN: u32 = 15;
const DRIVE_UL_PIN: u32 = 13;

const AF_TIM1_PA: u32 = 6;
const AF_TIM1_N: u32 = 4;

const VBUS_ADC_CHANNEL: u32 = 1;
const OP1_OUT_ADC_CHANNEL: u32 = 3;
const OP2_OUT_ADC_CHANNEL: u32 = 3;
const OP3_OUT_ADC_CHANNEL: u32 = 12;
const TEMP_ADC_CHANNEL: u32 = 5;
const POT_ADC_CHANNEL: u32 = 11;

const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

// ------------------------------------------------------------
// BEMF sense (observe-only) — B-G431B-ESC1 "BEMF DETECTION" network
// ------------------------------------------------------------
// GPIO_BEMF (PB5): output-LOW enables the divider (PWM-ON sampling);
// INPUT / high-Z disables it (PWM-OFF, ground-referenced sampling).
// At our low duty we use PWM-OFF sampling -> PB5 held as INPUT.
//
// Phase -> tap -> pin -> ADC2 channel (confirm vs DS12589 on first run):
//   U / OUT1 -> BEMF1 -> PA4  -> ADC2_IN17
//   V / OUT2 -> BEMF2 -> PC4  -> ADC2_IN5
//   W / OUT3 -> BEMF3 -> PB11 -> ADC2_IN14

const GPIO_BEMF_PIN: u32 = 5; // PB5

const BEMF1_PIN: u32 = 4; // PA4
const BEMF2_PIN: u32 = 4; // PC4
const BEMF3_PIN: u32 = 11; // PB11

const BEMF1_ADC_CHANNEL: u32 = 17; // ADC2_IN17 (PA4, U)
const BEMF2_ADC_CHANNEL: u32 = 5; // ADC2_IN5  (PC4, V)
const BEMF3_ADC_CHANNEL: u32 = 14; // ADC2_IN14 (PB11, W)

const BEMF_SAMPLE_BITS: u32 = 0b011; // 24.5 ADC cycles (fast, fits off-window)
const BEMF_BLANK_DELAY: u32 = 30_000; // post-commutation demag before sampling
const BEMF_SAMPLES_PER_STEP: usize = 4;

// ------------------------------------------------------------
// State definitions
// ------------------------------------------------------------

#[derive(Copy, Clone)]
enum DriveState {
    IdleAllOff,
    BootstrapUL,
    BootstrapVL,
    BootstrapWL,
    VectorUhVl,
    VectorUhWl,
    VectorVhWl,
    VectorVhUl,
    VectorWhUl,
    VectorWhVl,
    DeadtimeAllOff,
    FaultAllOff,
}

impl DriveState {
    fn name(self) -> &'static str {
        match self {
            DriveState::IdleAllOff => "idle_all_off",
            DriveState::BootstrapUL => "bootstrap_ul",
            DriveState::BootstrapVL => "bootstrap_vl",
            DriveState::BootstrapWL => "bootstrap_wl",
            DriveState::VectorUhVl => "vector_uh_vl",
            DriveState::VectorUhWl => "vector_uh_wl",
            DriveState::VectorVhWl => "vector_vh_wl",
            DriveState::VectorVhUl => "vector_vh_ul",
            DriveState::VectorWhUl => "vector_wh_ul",
            DriveState::VectorWhVl => "vector_wh_vl",
            DriveState::DeadtimeAllOff => "deadtime_all_off",
            DriveState::FaultAllOff => "fault_all_off",
        }
    }

    fn expected_ccer(self) -> u32 {
        match self {
            DriveState::IdleAllOff => TIM1_CCER_ALL_OFF_FORCED_LOW,
            DriveState::BootstrapUL => TIM1_CCER_BOOTSTRAP_U_LOW,
            DriveState::BootstrapVL => TIM1_CCER_BOOTSTRAP_V_LOW,
            DriveState::BootstrapWL => TIM1_CCER_BOOTSTRAP_W_LOW,
            DriveState::VectorUhVl => TIM1_CCER_VECTOR_UH_VL,
            DriveState::VectorUhWl => TIM1_CCER_VECTOR_UH_WL,
            DriveState::VectorVhWl => TIM1_CCER_VECTOR_VH_WL,
            DriveState::VectorVhUl => TIM1_CCER_VECTOR_VH_UL,
            DriveState::VectorWhUl => TIM1_CCER_VECTOR_WH_UL,
            DriveState::VectorWhVl => TIM1_CCER_VECTOR_WH_VL,
            DriveState::DeadtimeAllOff => TIM1_CCER_ALL_OFF_FORCED_LOW,
            DriveState::FaultAllOff => TIM1_CCER_ALL_OFF_FORCED_LOW,
        }
    }

    fn expected_pins(self) -> (u32, u32, u32, u32, u32, u32) {
        match self {
            DriveState::IdleAllOff => (0, 0, 0, 0, 0, 0),
            DriveState::BootstrapUL => (0, 1, 0, 0, 0, 0),
            DriveState::BootstrapVL => (0, 0, 0, 1, 0, 0),
            DriveState::BootstrapWL => (0, 0, 0, 0, 0, 1),
            DriveState::VectorUhVl => (1, 0, 0, 1, 0, 0),
            DriveState::VectorUhWl => (1, 0, 0, 0, 0, 1),
            DriveState::VectorVhWl => (0, 0, 1, 0, 0, 1),
            DriveState::VectorVhUl => (0, 1, 1, 0, 0, 0),
            DriveState::VectorWhUl => (0, 1, 0, 0, 1, 0),
            DriveState::VectorWhVl => (0, 0, 0, 1, 1, 0),
            DriveState::DeadtimeAllOff => (0, 0, 0, 0, 0, 0),
            DriveState::FaultAllOff => (0, 0, 0, 0, 0, 0),
        }
    }

    fn ch1_mode(self) -> u32 {
        match self {
            DriveState::VectorUhVl | DriveState::VectorUhWl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    fn ch2_mode(self) -> u32 {
        match self {
            DriveState::VectorVhWl | DriveState::VectorVhUl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    fn ch3_mode(self) -> u32 {
        match self {
            DriveState::VectorWhUl | DriveState::VectorWhVl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    fn active_led(self) -> bool {
        match self {
            DriveState::BootstrapUL
            | DriveState::BootstrapVL
            | DriveState::BootstrapWL
            | DriveState::VectorUhVl
            | DriveState::VectorUhWl
            | DriveState::VectorVhWl
            | DriveState::VectorVhUl
            | DriveState::VectorWhUl
            | DriveState::VectorWhVl => true,
            _ => false,
        }
    }
}

// ------------------------------------------------------------
// Readback structures
// ------------------------------------------------------------

#[derive(Copy, Clone)]
struct DriveReadback {
    uh_pin: u32,
    ul_pin: u32,
    vh_pin: u32,
    vl_pin: u32,
    wh_pin: u32,
    wl_pin: u32,
    af_ok: u32,
}

#[derive(Copy, Clone)]
struct Tim1Readback {
    cnt_a: u32,
    cnt_b: u32,
    ccer: u32,
    bdtr: u32,
    ccmr1: u32,
    ccmr2: u32,
    moe: u32,
    forced_modes_ok: u32,
    tim1_basic_ok: u32,
}

#[derive(Copy, Clone)]
struct AdcSnapshot {
    pot_raw: u16,
    temp_raw: u16,
    vbus_raw: u16,
    op1_raw: u16,
    op2_raw: u16,
    op3_raw: u16,
    timeout: u32,
}

// ------------------------------------------------------------
// GPIO helpers
// ------------------------------------------------------------

fn read_pin(idr: *const u32, pin: u32) -> u32 {
    let value = unsafe { read_volatile(idr) };

    if (value & (1 << pin)) != 0 {
        1
    } else {
        0
    }
}

fn set_pin_mode(moder: *mut u32, pin: u32, mode: u32) {
    unsafe {
        let mut value = read_volatile(moder);
        value &= !(0b11 << (pin * 2));
        value |= mode << (pin * 2);
        write_volatile(moder, value);
    }
}

fn get_pin_mode(moder: *mut u32, pin: u32) -> u32 {
    unsafe { (read_volatile(moder) >> (pin * 2)) & 0b11 }
}

fn set_pin_af(afrl: *mut u32, afrh: *mut u32, pin: u32, af: u32) {
    unsafe {
        if pin < 8 {
            let shift = pin * 4;
            let mut value = read_volatile(afrl);
            value &= !(0b1111 << shift);
            value |= af << shift;
            write_volatile(afrl, value);
        } else {
            let shift = (pin - 8) * 4;
            let mut value = read_volatile(afrh);
            value &= !(0b1111 << shift);
            value |= af << shift;
            write_volatile(afrh, value);
        }
    }
}

fn get_pin_af(afrl: *mut u32, afrh: *mut u32, pin: u32) -> u32 {
    unsafe {
        if pin < 8 {
            let shift = pin * 4;
            (read_volatile(afrl) >> shift) & 0b1111
        } else {
            let shift = (pin - 8) * 4;
            (read_volatile(afrh) >> shift) & 0b1111
        }
    }
}

fn led_high() {
    unsafe {
        write_volatile(GPIOC_BSRR, 1 << STATUS_LED_PIN);
    }
}

fn led_low() {
    unsafe {
        write_volatile(GPIOC_BSRR, 1 << (STATUS_LED_PIN + 16));
    }
}

fn led_for_state(state: DriveState) {
    if state.active_led() {
        led_high();
    } else {
        led_low();
    }
}

fn button_pressed() -> bool {
    let idr = unsafe { read_volatile(GPIOC_IDR) };
    (idr & (1 << USER_BUTTON_PIN)) == 0
}

fn force_drive_output_latches_low() {
    unsafe {
        write_volatile(
            GPIOA_BSRR,
            (1 << (DRIVE_UH_PIN + 16))
                | (1 << (DRIVE_VH_PIN + 16))
                | (1 << (DRIVE_WH_PIN + 16))
                | (1 << (DRIVE_VL_PIN + 16)),
        );

        write_volatile(GPIOB_BSRR, 1 << (DRIVE_WL_PIN + 16));
        write_volatile(GPIOC_BSRR, 1 << (DRIVE_UL_PIN + 16));
    }
}

fn read_drive() -> DriveReadback {
    let uh_pin = read_pin(GPIOA_IDR, DRIVE_UH_PIN);
    let vh_pin = read_pin(GPIOA_IDR, DRIVE_VH_PIN);
    let wh_pin = read_pin(GPIOA_IDR, DRIVE_WH_PIN);
    let vl_pin = read_pin(GPIOA_IDR, DRIVE_VL_PIN);
    let wl_pin = read_pin(GPIOB_IDR, DRIVE_WL_PIN);
    let ul_pin = read_pin(GPIOC_IDR, DRIVE_UL_PIN);

    let uh_mode = get_pin_mode(GPIOA_MODER, DRIVE_UH_PIN);
    let vh_mode = get_pin_mode(GPIOA_MODER, DRIVE_VH_PIN);
    let wh_mode = get_pin_mode(GPIOA_MODER, DRIVE_WH_PIN);
    let vl_mode = get_pin_mode(GPIOA_MODER, DRIVE_VL_PIN);
    let wl_mode = get_pin_mode(GPIOB_MODER, DRIVE_WL_PIN);
    let ul_mode = get_pin_mode(GPIOC_MODER, DRIVE_UL_PIN);

    let uh_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_UH_PIN);
    let vh_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VH_PIN);
    let wh_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_WH_PIN);
    let vl_af = get_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VL_PIN);
    let wl_af = get_pin_af(GPIOB_AFRL, GPIOB_AFRH, DRIVE_WL_PIN);
    let ul_af = get_pin_af(GPIOC_AFRL, GPIOC_AFRH, DRIVE_UL_PIN);

    let af_ok = if uh_mode == 0b10
        && vh_mode == 0b10
        && wh_mode == 0b10
        && vl_mode == 0b10
        && wl_mode == 0b10
        && ul_mode == 0b10
        && uh_af == AF_TIM1_PA
        && vh_af == AF_TIM1_PA
        && wh_af == AF_TIM1_PA
        && vl_af == AF_TIM1_PA
        && wl_af == AF_TIM1_N
        && ul_af == AF_TIM1_N
    {
        1
    } else {
        0
    };

    DriveReadback {
        uh_pin,
        ul_pin,
        vh_pin,
        vl_pin,
        wh_pin,
        wl_pin,
        af_ok,
    }
}

fn pins_match_state(drive: DriveReadback, state: DriveState) -> u32 {
    let expected = state.expected_pins();

    if drive.uh_pin == expected.0
        && drive.ul_pin == expected.1
        && drive.vh_pin == expected.2
        && drive.vl_pin == expected.3
        && drive.wh_pin == expected.4
        && drive.wl_pin == expected.5
    {
        1
    } else {
        0
    }
}

fn no_phase_overlap(drive: DriveReadback) -> u32 {
    if (drive.uh_pin == 1 && drive.ul_pin == 1)
        || (drive.vh_pin == 1 && drive.vl_pin == 1)
        || (drive.wh_pin == 1 && drive.wl_pin == 1)
    {
        0
    } else {
        1
    }
}

fn active_pin_count(drive: DriveReadback) -> u32 {
    drive.uh_pin + drive.ul_pin + drive.vh_pin + drive.vl_pin + drive.wh_pin + drive.wl_pin
}

// ------------------------------------------------------------
// Delay helpers
// ------------------------------------------------------------

fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

fn delay_while_button_held(total_cycles: u32) -> bool {
    let mut remaining = total_cycles;

    while remaining > 0 {
        if !button_pressed() {
            apply_state(DriveState::IdleAllOff);
            return false;
        }

        let step = if remaining > RELEASE_CHECK_CHUNK {
            RELEASE_CHECK_CHUNK
        } else {
            remaining
        };

        delay_cycles(step);
        remaining -= step;
    }

    true
}

// ------------------------------------------------------------
// TIM1 helpers
// ------------------------------------------------------------

fn set_tim1_modes_for_state(state: DriveState) {
    unsafe {
        let ccmr1 =
            (state.ch1_mode() << 4) |
            (state.ch2_mode() << 12);

        let ccmr2 = state.ch3_mode() << 4;

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);
    }
}

// Non-PWM states (idle, bootstrap precharge, deadtime, fault).
// High side forced active/inactive, CCR held at 0. Used for the
// startup readback, bootstrap precharge and all dead-man stops.
fn apply_state(state: DriveState) {
    unsafe {
        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        set_tim1_modes_for_state(state);

        write_volatile(TIM1_CCER, state.expected_ccer());

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr |= TIM1_BDTR_OSSI;
        bdtr |= TIM1_BDTR_OSSR;
        bdtr |= TIM1_BDTR_MOE;
        write_volatile(TIM1_BDTR, bdtr);
    }

    led_for_state(state);
}

// PWM vector apply: the high-side channel the commutation table marks
// FORCED_ACTIVE is switched to PWM_MODE_1 and loaded with `duty`; the
// low-side-active channel stays FORCED_INACTIVE (its complementary low
// FET on); the floating channel stays disabled in CCER. The CCER value
// is the same confirmed table entry used by apply_state. The PWMing
// phase's low FET is never actively driven, so off-time current
// freewheels through its body diode and recharges the bootstrap cap.
fn apply_pwm_vector(state: DriveState, duty: u32) {
    let ch1_mode = if state.ch1_mode() == TIM1_CCMR_FORCED_ACTIVE {
        TIM1_CCMR_PWM_MODE_1
    } else {
        TIM1_CCMR_FORCED_INACTIVE
    };
    let ch2_mode = if state.ch2_mode() == TIM1_CCMR_FORCED_ACTIVE {
        TIM1_CCMR_PWM_MODE_1
    } else {
        TIM1_CCMR_FORCED_INACTIVE
    };
    let ch3_mode = if state.ch3_mode() == TIM1_CCMR_FORCED_ACTIVE {
        TIM1_CCMR_PWM_MODE_1
    } else {
        TIM1_CCMR_FORCED_INACTIVE
    };

    unsafe {
        write_volatile(
            TIM1_CCR1,
            if ch1_mode == TIM1_CCMR_PWM_MODE_1 { duty } else { 0 },
        );
        write_volatile(
            TIM1_CCR2,
            if ch2_mode == TIM1_CCMR_PWM_MODE_1 { duty } else { 0 },
        );
        write_volatile(
            TIM1_CCR3,
            if ch3_mode == TIM1_CCMR_PWM_MODE_1 { duty } else { 0 },
        );

        let ccmr1 = (ch1_mode << 4) | (ch2_mode << 12);
        let ccmr2 = ch3_mode << 4;
        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);

        write_volatile(TIM1_CCER, state.expected_ccer());

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr |= TIM1_BDTR_OSSI;
        bdtr |= TIM1_BDTR_OSSR;
        bdtr |= TIM1_BDTR_MOE;
        write_volatile(TIM1_BDTR, bdtr);
    }

    led_for_state(state);
}

fn read_tim1_for_state(state: DriveState) -> Tim1Readback {
    unsafe {
        let cnt_a = read_volatile(TIM1_CNT);
        delay_cycles(16_000);
        let cnt_b = read_volatile(TIM1_CNT);

        let ccr1 = read_volatile(TIM1_CCR1);
        let ccr2 = read_volatile(TIM1_CCR2);
        let ccr3 = read_volatile(TIM1_CCR3);
        let ccer = read_volatile(TIM1_CCER);
        let bdtr = read_volatile(TIM1_BDTR);
        let cr2 = read_volatile(TIM1_CR2);
        let ccmr1 = read_volatile(TIM1_CCMR1);
        let ccmr2 = read_volatile(TIM1_CCMR2);

        let counting = if cnt_a != cnt_b { 1 } else { 0 };
        let moe = if (bdtr & TIM1_BDTR_MOE) != 0 { 1 } else { 0 };
        let ossi = if (bdtr & TIM1_BDTR_OSSI) != 0 { 1 } else { 0 };
        let ossr = if (bdtr & TIM1_BDTR_OSSR) != 0 { 1 } else { 0 };

        let oc1m = (ccmr1 >> 4) & 0b111;
        let oc2m = (ccmr1 >> 12) & 0b111;
        let oc3m = (ccmr2 >> 4) & 0b111;

        let forced_modes_ok = if oc1m == state.ch1_mode()
            && oc2m == state.ch2_mode()
            && oc3m == state.ch3_mode()
        {
            1
        } else {
            0
        };

        let idle_bits = (cr2 >> 8) & 0b11_1111;

        let tim1_basic_ok = if counting == 1
            && ccer == state.expected_ccer()
            && moe == 1
            && ossi == 1
            && ossr == 1
            && forced_modes_ok == 1
            && idle_bits == 0
            && ccr1 == 0
            && ccr2 == 0
            && ccr3 == 0
        {
            1
        } else {
            0
        };

        Tim1Readback {
            cnt_a,
            cnt_b,
            ccer,
            bdtr,
            ccmr1,
            ccmr2,
            moe,
            forced_modes_ok,
            tim1_basic_ok,
        }
    }
}

fn setup_tim1_base() {
    unsafe {
        let apb2enr = read_volatile(RCC_APB2ENR);
        write_volatile(RCC_APB2ENR, apb2enr | RCC_APB2ENR_TIM1EN);

        delay_cycles(8_000);

        write_volatile(TIM1_CR1, 0);
        write_volatile(TIM1_CCER, 0);

        let mut cr2 = read_volatile(TIM1_CR2);
        cr2 &= !(0b11_1111 << 8);
        write_volatile(TIM1_CR2, cr2);

        write_volatile(TIM1_PSC, TIM1_TEST_PSC);
        write_volatile(TIM1_ARR, TIM1_TEST_ARR);
        write_volatile(TIM1_RCR, 0);

        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        set_tim1_modes_for_state(DriveState::IdleAllOff);

        write_volatile(TIM1_EGR, TIM1_EGR_UG);
        write_volatile(TIM1_CR1, TIM1_CR1_ARPE | TIM1_CR1_CEN);

        apply_state(DriveState::IdleAllOff);
    }
}

// ------------------------------------------------------------
// ADC helpers
// ------------------------------------------------------------

fn adc_select_channel(adc_base: usize, channel: u32) {
    unsafe {
        let sqr1 = channel << 6;
        write_volatile(adc_sqr1(adc_base), sqr1);
    }
}

fn adc_set_sample_time(adc_base: usize, channel: u32) {
    unsafe {
        let sample_bits: u32 = 0b111;

        if channel <= 9 {
            let shift = channel * 3;
            let mut smpr1 = read_volatile(adc_smpr1(adc_base));
            smpr1 &= !(0b111 << shift);
            smpr1 |= sample_bits << shift;
            write_volatile(adc_smpr1(adc_base), smpr1);
        } else {
            let shift = (channel - 10) * 3;
            let mut smpr2 = read_volatile(adc_smpr2(adc_base));
            smpr2 &= !(0b111 << shift);
            smpr2 |= sample_bits << shift;
            write_volatile(adc_smpr2(adc_base), smpr2);
        }
    }
}

// Same as adc_set_sample_time but with a caller-chosen sample-time
// field. Used for the BEMF channels, which need a short sample time
// so the conversion fits inside the PWM-off window.
fn adc_set_sample_time_bits(adc_base: usize, channel: u32, bits: u32) {
    unsafe {
        if channel <= 9 {
            let shift = channel * 3;
            let mut smpr1 = read_volatile(adc_smpr1(adc_base));
            smpr1 &= !(0b111 << shift);
            smpr1 |= (bits & 0b111) << shift;
            write_volatile(adc_smpr1(adc_base), smpr1);
        } else {
            let shift = (channel - 10) * 3;
            let mut smpr2 = read_volatile(adc_smpr2(adc_base));
            smpr2 &= !(0b111 << shift);
            smpr2 |= (bits & 0b111) << shift;
            write_volatile(adc_smpr2(adc_base), smpr2);
        }
    }
}

fn adc_set_single_ended(adc_base: usize, channel: u32) {
    unsafe {
        let mut difsel = read_volatile(adc_difsel(adc_base));
        difsel &= !(1 << channel);
        write_volatile(adc_difsel(adc_base), difsel);
    }
}

fn adc_read_channel_raw(adc_base: usize, channel: u32) -> u16 {
    unsafe {
        adc_select_channel(adc_base, channel);

        write_volatile(adc_isr(adc_base), ADC_ISR_EOC | ADC_ISR_EOS);

        let cr = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr | ADC_CR_ADSTART);

        for _ in 0..1_000_000 {
            let isr = read_volatile(adc_isr(adc_base));

            if (isr & ADC_ISR_EOC) != 0 {
                let raw = read_volatile(adc_dr(adc_base)) & 0x0FFF;
                return raw as u16;
            }
        }
    }

    ADC_TIMEOUT_VALUE
}

fn adc_delta(current: u16, baseline: u16) -> i32 {
    (current as i32) - (baseline as i32)
}

fn read_adc_snapshot() -> AdcSnapshot {
    let pot_raw = adc_read_channel_raw(ADC1_BASE, POT_ADC_CHANNEL);
    let temp_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let op1_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    let op2_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    let op3_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    let timeout = if pot_raw == ADC_TIMEOUT_VALUE
        || temp_raw == ADC_TIMEOUT_VALUE
        || vbus_raw == ADC_TIMEOUT_VALUE
        || op1_raw == ADC_TIMEOUT_VALUE
        || op2_raw == ADC_TIMEOUT_VALUE
        || op3_raw == ADC_TIMEOUT_VALUE
    {
        1
    } else {
        0
    };

    AdcSnapshot {
        pot_raw,
        temp_raw,
        vbus_raw,
        op1_raw,
        op2_raw,
        op3_raw,
        timeout,
    }
}

// ------------------------------------------------------------
// Peripheral setup
// ------------------------------------------------------------

fn setup_gpio_base() {
    unsafe {
        let rcc_ahb2enr = read_volatile(RCC_AHB2ENR);
        write_volatile(
            RCC_AHB2ENR,
            rcc_ahb2enr
                | RCC_AHB2ENR_GPIOAEN
                | RCC_AHB2ENR_GPIOBEN
                | RCC_AHB2ENR_GPIOCEN
                | RCC_AHB2ENR_ADC12EN,
        );

        delay_cycles(8_000);

        force_drive_output_latches_low();

        set_pin_mode(GPIOA_MODER, VBUS_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP1_OUT_PIN, 0b11);
        set_pin_mode(GPIOA_MODER, OP2_OUT_PIN, 0b11);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (VBUS_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP1_OUT_PIN * 2));
        gpioa_pupdr &= !(0b11 << (OP2_OUT_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ascr = read_volatile(GPIOA_ASCR);
        gpioa_ascr |= 1 << VBUS_PIN;
        gpioa_ascr |= 1 << OP1_OUT_PIN;
        gpioa_ascr |= 1 << OP2_OUT_PIN;
        write_volatile(GPIOA_ASCR, gpioa_ascr);

        set_pin_mode(GPIOB_MODER, OP3_OUT_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, POT_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, TEMP_PIN, 0b11);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (OP3_OUT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (POT_PIN * 2));
        gpiob_pupdr &= !(0b11 << (TEMP_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ascr = read_volatile(GPIOB_ASCR);
        gpiob_ascr |= 1 << OP3_OUT_PIN;
        gpiob_ascr |= 1 << POT_PIN;
        gpiob_ascr |= 1 << TEMP_PIN;
        write_volatile(GPIOB_ASCR, gpiob_ascr);

        set_pin_mode(GPIOC_MODER, STATUS_LED_PIN, 0b01);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << STATUS_LED_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        set_pin_mode(GPIOC_MODER, USER_BUTTON_PIN, 0b00);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_pupdr &= !(0b11 << (USER_BUTTON_PIN * 2));
        gpioc_pupdr |= 0b01 << (USER_BUTTON_PIN * 2);
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        led_low();
    }
}

fn setup_drive_pins_tim1_af() {
    unsafe {
        write_volatile(TIM1_CCER, 0);

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr &= !TIM1_BDTR_MOE;
        bdtr |= TIM1_BDTR_OSSI | TIM1_BDTR_OSSR;
        write_volatile(TIM1_BDTR, bdtr);

        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        force_drive_output_latches_low();

        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_UH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_WH_PIN, AF_TIM1_PA);
        set_pin_af(GPIOA_AFRL, GPIOA_AFRH, DRIVE_VL_PIN, AF_TIM1_PA);

        set_pin_mode(GPIOA_MODER, DRIVE_UH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_VH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_WH_PIN, 0b10);
        set_pin_mode(GPIOA_MODER, DRIVE_VL_PIN, 0b10);

        let mut gpioa_otyper = read_volatile(GPIOA_OTYPER);
        gpioa_otyper &= !(1 << DRIVE_UH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VH_PIN);
        gpioa_otyper &= !(1 << DRIVE_WH_PIN);
        gpioa_otyper &= !(1 << DRIVE_VL_PIN);
        write_volatile(GPIOA_OTYPER, gpioa_otyper);

        let mut gpioa_pupdr = read_volatile(GPIOA_PUPDR);
        gpioa_pupdr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_pupdr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_PUPDR, gpioa_pupdr);

        let mut gpioa_ospeedr = read_volatile(GPIOA_OSPEEDR);
        gpioa_ospeedr &= !(0b11 << (DRIVE_UH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_WH_PIN * 2));
        gpioa_ospeedr &= !(0b11 << (DRIVE_VL_PIN * 2));
        write_volatile(GPIOA_OSPEEDR, gpioa_ospeedr);

        set_pin_af(GPIOB_AFRL, GPIOB_AFRH, DRIVE_WL_PIN, AF_TIM1_N);
        set_pin_mode(GPIOB_MODER, DRIVE_WL_PIN, 0b10);

        let mut gpiob_otyper = read_volatile(GPIOB_OTYPER);
        gpiob_otyper &= !(1 << DRIVE_WL_PIN);
        write_volatile(GPIOB_OTYPER, gpiob_otyper);

        let mut gpiob_pupdr = read_volatile(GPIOB_PUPDR);
        gpiob_pupdr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_PUPDR, gpiob_pupdr);

        let mut gpiob_ospeedr = read_volatile(GPIOB_OSPEEDR);
        gpiob_ospeedr &= !(0b11 << (DRIVE_WL_PIN * 2));
        write_volatile(GPIOB_OSPEEDR, gpiob_ospeedr);

        set_pin_af(GPIOC_AFRL, GPIOC_AFRH, DRIVE_UL_PIN, AF_TIM1_N);
        set_pin_mode(GPIOC_MODER, DRIVE_UL_PIN, 0b10);

        let mut gpioc_otyper = read_volatile(GPIOC_OTYPER);
        gpioc_otyper &= !(1 << DRIVE_UL_PIN);
        write_volatile(GPIOC_OTYPER, gpioc_otyper);

        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);

        let mut gpioc_ospeedr = read_volatile(GPIOC_OSPEEDR);
        gpioc_ospeedr &= !(0b11 << (DRIVE_UL_PIN * 2));
        write_volatile(GPIOC_OSPEEDR, gpioc_ospeedr);

        apply_state(DriveState::IdleAllOff);
    }
}

// GPIO_BEMF (PB5) and the three BEMF sense pins. PB5 is held as an
// input (high-Z) so the on-board divider is DISABLED, which is the
// correct configuration for ground-referenced PWM-off sampling at low
// duty. The sense pins go to analog mode with the analog switch closed.
fn setup_bemf_pins() {
    unsafe {
        // GPIO_BEMF = PB5 -> input, no pull (divider disabled).
        set_pin_mode(GPIOB_MODER, GPIO_BEMF_PIN, 0b00);
        let mut pb5_pupdr = read_volatile(GPIOB_PUPDR);
        pb5_pupdr &= !(0b11 << (GPIO_BEMF_PIN * 2));
        write_volatile(GPIOB_PUPDR, pb5_pupdr);

        // BEMF1 = PA4, BEMF2 = PC4, BEMF3 = PB11 -> analog, no pull.
        set_pin_mode(GPIOA_MODER, BEMF1_PIN, 0b11);
        set_pin_mode(GPIOC_MODER, BEMF2_PIN, 0b11);
        set_pin_mode(GPIOB_MODER, BEMF3_PIN, 0b11);

        let mut a_pupdr = read_volatile(GPIOA_PUPDR);
        a_pupdr &= !(0b11 << (BEMF1_PIN * 2));
        write_volatile(GPIOA_PUPDR, a_pupdr);

        let mut c_pupdr = read_volatile(GPIOC_PUPDR);
        c_pupdr &= !(0b11 << (BEMF2_PIN * 2));
        write_volatile(GPIOC_PUPDR, c_pupdr);

        let mut b_pupdr = read_volatile(GPIOB_PUPDR);
        b_pupdr &= !(0b11 << (BEMF3_PIN * 2));
        write_volatile(GPIOB_PUPDR, b_pupdr);

        // Close the analog switch for each sense pin (ASCR).
        let mut a_ascr = read_volatile(GPIOA_ASCR);
        a_ascr |= 1 << BEMF1_PIN;
        write_volatile(GPIOA_ASCR, a_ascr);

        let mut c_ascr = read_volatile(GPIOC_ASCR);
        c_ascr |= 1 << BEMF2_PIN;
        write_volatile(GPIOC_ASCR, c_ascr);

        let mut b_ascr = read_volatile(GPIOB_ASCR);
        b_ascr |= 1 << BEMF3_PIN;
        write_volatile(GPIOB_ASCR, b_ascr);
    }
}

fn setup_adc12_common_clock() {
    unsafe {
        let mut ccr = read_volatile(ADC12_CCR);
        ccr &= !(0b11 << 16);
        ccr |= 0b01 << 16;
        write_volatile(ADC12_CCR, ccr);
    }
}

fn setup_single_adc(adc_base: usize) -> u32 {
    let mut status: u32 = 0;

    unsafe {
        let mut cr = read_volatile(adc_cr(adc_base));
        cr &= !ADC_CR_DEEPPWD;
        cr |= ADC_CR_ADVREGEN;
        write_volatile(adc_cr(adc_base), cr);

        delay_cycles(160_000);

        write_volatile(adc_cfgr(adc_base), 0);

        let cr_before_cal = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr_before_cal | ADC_CR_ADCAL);

        let mut cal_done = false;
        for _ in 0..1_000_000 {
            if (read_volatile(adc_cr(adc_base)) & ADC_CR_ADCAL) == 0 {
                cal_done = true;
                break;
            }
        }

        if !cal_done {
            status |= 1 << 0;
        }

        write_volatile(adc_isr(adc_base), ADC_ISR_ADRDY);

        let cr_before_enable = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr_before_enable | ADC_CR_ADEN);

        let mut ready = false;
        for _ in 0..1_000_000 {
            if (read_volatile(adc_isr(adc_base)) & ADC_ISR_ADRDY) != 0 {
                ready = true;
                break;
            }
        }

        if !ready {
            status |= 1 << 1;
        }
    }

    status
}

fn setup_adc1_channels() {
    adc_set_single_ended(ADC1_BASE, VBUS_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, TEMP_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, POT_ADC_CHANNEL);
    adc_set_single_ended(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    adc_set_sample_time(ADC1_BASE, VBUS_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, TEMP_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, POT_ADC_CHANNEL);
    adc_set_sample_time(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    adc_select_channel(ADC1_BASE, POT_ADC_CHANNEL);
}

fn setup_adc2_channels() {
    adc_set_single_ended(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    adc_set_sample_time(ADC2_BASE, OP2_OUT_ADC_CHANNEL);

    // BEMF sense channels on ADC2 (fast sample time for in-window reads).
    adc_set_single_ended(ADC2_BASE, BEMF1_ADC_CHANNEL);
    adc_set_single_ended(ADC2_BASE, BEMF2_ADC_CHANNEL);
    adc_set_single_ended(ADC2_BASE, BEMF3_ADC_CHANNEL);
    adc_set_sample_time_bits(ADC2_BASE, BEMF1_ADC_CHANNEL, BEMF_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC2_BASE, BEMF2_ADC_CHANNEL, BEMF_SAMPLE_BITS);
    adc_set_sample_time_bits(ADC2_BASE, BEMF3_ADC_CHANNEL, BEMF_SAMPLE_BITS);

    adc_select_channel(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
}

fn setup_adc_for_board_monitor() -> (u32, u32) {
    setup_adc12_common_clock();

    setup_adc1_channels();
    let adc1_status = setup_single_adc(ADC1_BASE);

    setup_adc2_channels();
    let adc2_status = setup_single_adc(ADC2_BASE);

    (adc1_status, adc2_status)
}

// ------------------------------------------------------------
// Heavy verification log (startup / idle / fault only)
// ------------------------------------------------------------

fn log_state(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    vector_delay: u32,
    state: DriveState,
    baseline: AdcSnapshot,
) -> u32 {
    delay_cycles(STATE_SETTLE_DELAY);

    let drive = read_drive();
    let tim1 = read_tim1_for_state(state);
    let adc = read_adc_snapshot();

    let pins_ok = pins_match_state(drive, state);
    let no_overlap = no_phase_overlap(drive);
    let active_count = active_pin_count(drive);

    let expected = state.expected_pins();
    let expected_active_count =
        expected.0 + expected.1 + expected.2 + expected.3 + expected.4 + expected.5;

    let active_count_ok = if active_count == expected_active_count { 1 } else { 0 };
    let temp_delta = adc_delta(adc.temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let state_ok = if drive.af_ok == 1
        && pins_ok == 1
        && no_overlap == 1
        && active_count_ok == 1
        && tim1.tim1_basic_ok == 1
        && adc.timeout == 0
        && temp_ok == 1
    {
        1
    } else {
        0
    };

    rprintln!(
        "run={} step={} erev={} delay={} state={} state_ok={} button={} af_ok={} pins_ok={} no_phase_overlap={} active_count={} active_count_ok={} tim1_ok={} expected_ccer={} tim1_ccer={} tim1_moe={} forced_modes_ok={} UH={} UL={} VH={} VL={} WH={} WL={} cnt_a={} cnt_b={} ccmr1={} ccmr2={} bdtr={} pot_raw={} temp_raw={} temp_delta={} temp_ok={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} timeout={}",
        run_id,
        step_index,
        electrical_rev,
        vector_delay,
        state.name(),
        state_ok,
        if button_pressed() { 1 } else { 0 },
        drive.af_ok,
        pins_ok,
        no_overlap,
        active_count,
        active_count_ok,
        tim1.tim1_basic_ok,
        state.expected_ccer(),
        tim1.ccer,
        tim1.moe,
        tim1.forced_modes_ok,
        drive.uh_pin,
        drive.ul_pin,
        drive.vh_pin,
        drive.vl_pin,
        drive.wh_pin,
        drive.wl_pin,
        tim1.cnt_a,
        tim1.cnt_b,
        tim1.ccmr1,
        tim1.ccmr2,
        tim1.bdtr,
        adc.pot_raw,
        adc.temp_raw,
        temp_delta,
        temp_ok,
        adc.vbus_raw,
        adc_delta(adc.vbus_raw, baseline.vbus_raw),
        adc.op1_raw,
        adc_delta(adc.op1_raw, baseline.op1_raw),
        adc.op2_raw,
        adc_delta(adc.op2_raw, baseline.op2_raw),
        adc.op3_raw,
        adc_delta(adc.op3_raw, baseline.op3_raw),
        adc.timeout
    );

    state_ok
}

// ------------------------------------------------------------
// Compact run heartbeat (once per electrical rev during the ramp)
// ------------------------------------------------------------
// High-side pins are PWMing during a vector, so an instantaneous IDR
// read of a high pin is non-deterministic and is reported, not
// asserted. The health gate uses only deterministic / safety signals:
// AF mapping, no high+low overlap, temp delta, ADC timeout.

fn log_heartbeat(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    vector_delay: u32,
    duty: u32,
    state: DriveState,
    baseline: AdcSnapshot,
) -> u32 {
    let drive = read_drive();
    let adc = read_adc_snapshot();

    let no_overlap = no_phase_overlap(drive);
    let temp_delta = adc_delta(adc.temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let health_ok = if drive.af_ok == 1
        && no_overlap == 1
        && adc.timeout == 0
        && temp_ok == 1
    {
        1
    } else {
        0
    };

    rprintln!(
        "hb run={} step={} erev={} delay={} duty={} vector={} health_ok={} button={} af_ok={} no_phase_overlap={} UH={} UL={} VH={} VL={} WH={} WL={} vbus_raw={} vbus_delta={} temp_raw={} temp_delta={} temp_ok={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} timeout={}",
        run_id,
        step_index,
        electrical_rev,
        vector_delay,
        duty,
        state.name(),
        health_ok,
        if button_pressed() { 1 } else { 0 },
        drive.af_ok,
        no_overlap,
        drive.uh_pin,
        drive.ul_pin,
        drive.vh_pin,
        drive.vl_pin,
        drive.wh_pin,
        drive.wl_pin,
        adc.vbus_raw,
        adc_delta(adc.vbus_raw, baseline.vbus_raw),
        adc.temp_raw,
        temp_delta,
        temp_ok,
        adc.op1_raw,
        adc_delta(adc.op1_raw, baseline.op1_raw),
        adc.op2_raw,
        adc_delta(adc.op2_raw, baseline.op2_raw),
        adc.op3_raw,
        adc_delta(adc.op3_raw, baseline.op3_raw),
        adc.timeout
    );

    health_ok
}

// ------------------------------------------------------------
// Ramp scheduling (UNCHANGED)
// ------------------------------------------------------------

fn ramp_delay_for_step(vector_step: u32) -> (u32, u32) {
    let electrical_rev = vector_step / 6;
    let reduction = electrical_rev * RAMP_DECREMENT_PER_ELECTRICAL_REV;

    let delay = if RAMP_START_VECTOR_DELAY > reduction {
        RAMP_START_VECTOR_DELAY - reduction
    } else {
        RAMP_MIN_VECTOR_DELAY
    };

    if delay < RAMP_MIN_VECTOR_DELAY {
        (RAMP_MIN_VECTOR_DELAY, electrical_rev)
    } else {
        (delay, electrical_rev)
    }
}

// ------------------------------------------------------------
// BEMF sampling (observe-only)
// ------------------------------------------------------------

// The floating phase is the one not driven this vector; return its
// ADC2 BEMF channel and a label for the log.
fn floating_bemf_channel(state: DriveState) -> (u32, &'static str) {
    match state {
        DriveState::VectorUhVl | DriveState::VectorVhUl => (BEMF3_ADC_CHANNEL, "W"),
        DriveState::VectorUhWl | DriveState::VectorWhUl => (BEMF2_ADC_CHANNEL, "V"),
        DriveState::VectorVhWl | DriveState::VectorWhVl => (BEMF1_ADC_CHANNEL, "U"),
        _ => (BEMF1_ADC_CHANNEL, "?"),
    }
}

// Sample the floating-phase BEMF inside the PWM-off window. The high
// side PWMs high while CNT < duty, so the off window is CNT from duty
// up to ARR; we wait for the late part of that window (leaving margin
// for the sample phase to finish before the counter wraps) then read.
fn read_bemf_floating(channel: u32, duty: u32) -> u16 {
    unsafe {
        let lo = duty + (TIM1_TEST_ARR - duty) / 2;
        let hi = if TIM1_TEST_ARR > 40 {
            TIM1_TEST_ARR - 40
        } else {
            TIM1_TEST_ARR
        };

        let mut guard: u32 = 0;
        loop {
            let cnt = read_volatile(TIM1_CNT);
            if cnt >= lo && cnt <= hi {
                break;
            }
            guard += 1;
            if guard > 2_000_000 {
                return ADC_TIMEOUT_VALUE;
            }
        }

        adc_select_channel(ADC2_BASE, channel);
        write_volatile(adc_isr(ADC2_BASE), ADC_ISR_EOC | ADC_ISR_EOS);
        let cr = read_volatile(adc_cr(ADC2_BASE));
        write_volatile(adc_cr(ADC2_BASE), cr | ADC_CR_ADSTART);

        for _ in 0..100_000 {
            if (read_volatile(adc_isr(ADC2_BASE)) & ADC_ISR_EOC) != 0 {
                return (read_volatile(adc_dr(ADC2_BASE)) & 0x0FFF) as u16;
            }
        }
    }

    ADC_TIMEOUT_VALUE
}

// One compact per-step line: the four floating-phase BEMF samples plus
// the safety fields. Returns the health gate (af / overlap / temp /
// timeout) — note this still cannot detect a rotor slip.
fn log_bemf_step(
    run_id: u32,
    step_index: u32,
    electrical_rev: u32,
    vector_delay: u32,
    duty: u32,
    state: DriveState,
    float_label: &str,
    samples: &[u16; BEMF_SAMPLES_PER_STEP],
    baseline: AdcSnapshot,
) -> u32 {
    let drive = read_drive();
    let temp_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let timeout = if temp_raw == ADC_TIMEOUT_VALUE
        || vbus_raw == ADC_TIMEOUT_VALUE
        || samples[0] == ADC_TIMEOUT_VALUE
        || samples[1] == ADC_TIMEOUT_VALUE
        || samples[2] == ADC_TIMEOUT_VALUE
        || samples[3] == ADC_TIMEOUT_VALUE
    {
        1
    } else {
        0
    };

    let no_overlap = no_phase_overlap(drive);
    let temp_delta = adc_delta(temp_raw, baseline.temp_raw);
    let temp_ok = if temp_delta < TEMP_DELTA_ABORT_RAW { 1 } else { 0 };

    let health_ok = if drive.af_ok == 1 && no_overlap == 1 && timeout == 0 && temp_ok == 1 {
        1
    } else {
        0
    };

    rprintln!(
        "bemf run={} step={} erev={} delay={} duty={} vector={} float_phase={} b0={} b1={} b2={} b3={} health_ok={} button={} af_ok={} no_phase_overlap={} vbus_raw={} vbus_delta={} temp_raw={} temp_delta={} timeout={}",
        run_id,
        step_index,
        electrical_rev,
        vector_delay,
        duty,
        state.name(),
        float_label,
        samples[0],
        samples[1],
        samples[2],
        samples[3],
        health_ok,
        if button_pressed() { 1 } else { 0 },
        drive.af_ok,
        no_overlap,
        vbus_raw,
        adc_delta(vbus_raw, baseline.vbus_raw),
        temp_raw,
        temp_delta,
        timeout
    );

    health_ok
}

// ------------------------------------------------------------
// PWM open-loop run: precharge -> align -> ramp
// ------------------------------------------------------------

fn run_pwm_openloop_ramp(run_id: u32, baseline: AdcSnapshot) {
    let sequence = [
        DriveState::VectorUhVl,
        DriveState::VectorUhWl,
        DriveState::VectorVhWl,
        DriveState::VectorVhUl,
        DriveState::VectorWhUl,
        DriveState::VectorWhVl,
    ];

    rprintln!(
        "pwm_run_start run={} hold_button_to_run release_to_stop align_duty={} run_start_duty={} run_max_duty={} inc_per_erev={} align_hold={} start_delay={} min_delay={} dec_per_erev={} max_steps={}",
        run_id,
        PWM_DUTY_ALIGN,
        PWM_DUTY_RUN_START,
        PWM_DUTY_RUN_MAX,
        PWM_DUTY_INC_PER_EREV,
        ALIGN_HOLD_DELAY,
        RAMP_START_VECTOR_DELAY,
        RAMP_MIN_VECTOR_DELAY,
        RAMP_DECREMENT_PER_ELECTRICAL_REV,
        MAX_VECTOR_STEPS_PER_HOLD
    );

    // One-time bootstrap precharge: turn each phase low FET on briefly
    // so all three high-side bootstrap caps are charged before the
    // first PWM vector. Reuses the confirmed Bootstrap*L states.
    apply_state(DriveState::BootstrapUL);
    if !delay_while_button_held(BOOTSTRAP_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }
    apply_state(DriveState::BootstrapVL);
    if !delay_while_button_held(BOOTSTRAP_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }
    apply_state(DriveState::BootstrapWL);
    if !delay_while_button_held(BOOTSTRAP_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }

    // Alignment: park the rotor on the first vector at align duty.
    apply_pwm_vector(sequence[0], PWM_DUTY_ALIGN);
    let align_health = log_heartbeat(
        run_id,
        0,
        0,
        ALIGN_HOLD_DELAY,
        PWM_DUTY_ALIGN,
        sequence[0],
        baseline,
    );
    if align_health != 1 {
        apply_state(DriveState::FaultAllOff);
        log_state(run_id, 0, 0, 0, DriveState::FaultAllOff, baseline);
        return;
    }
    if !delay_while_button_held(ALIGN_HOLD_DELAY) {
        apply_state(DriveState::IdleAllOff);
        return;
    }

    // Ramp: direct vector-to-vector PWM commutation, no dead windows.
    let mut vector_steps: u32 = 0;
    let mut cycle_ok: u32 = 1;

    while button_pressed() && vector_steps < MAX_VECTOR_STEPS_PER_HOLD {
        let index = (vector_steps % 6) as usize;
        let vector_state = sequence[index];
        let (vector_delay, electrical_rev) = ramp_delay_for_step(vector_steps);

        let duty_unclamped = PWM_DUTY_RUN_START + electrical_rev * PWM_DUTY_INC_PER_EREV;
        let duty = if duty_unclamped > PWM_DUTY_RUN_MAX {
            PWM_DUTY_RUN_MAX
        } else {
            duty_unclamped
        };

        apply_pwm_vector(vector_state, duty);

        let (bemf_chan, float_label) = floating_bemf_channel(vector_state);
        let mut samples: [u16; BEMF_SAMPLES_PER_STEP] =
            [ADC_TIMEOUT_VALUE; BEMF_SAMPLES_PER_STEP];

        // Post-commutation blanking (demag) before the BEMF is valid.
        if !delay_while_button_held(BEMF_BLANK_DELAY) {
            apply_state(DriveState::IdleAllOff);
            rprintln!(
                "pwm_run_stop run={} cycle_ok=1 vector_steps={} reason=button_released_mid_step",
                run_id,
                vector_steps
            );
            log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
            return;
        }

        // Sample the floating phase across the rest of the step, each
        // sample taken inside the PWM-off window. Total hold ~= vector_delay.
        let remaining = if vector_delay > BEMF_BLANK_DELAY {
            vector_delay - BEMF_BLANK_DELAY
        } else {
            0
        };
        let sub = remaining / (BEMF_SAMPLES_PER_STEP as u32);

        let mut released = false;
        let mut i = 0usize;
        while i < BEMF_SAMPLES_PER_STEP {
            if sub > 0 && !delay_while_button_held(sub) {
                released = true;
                break;
            }
            samples[i] = read_bemf_floating(bemf_chan, duty);
            i += 1;
        }

        if released {
            apply_state(DriveState::IdleAllOff);
            rprintln!(
                "pwm_run_stop run={} cycle_ok=1 vector_steps={} reason=button_released_mid_step",
                run_id,
                vector_steps
            );
            log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
            return;
        }

        let health_ok = log_bemf_step(
            run_id,
            vector_steps + 1,
            electrical_rev,
            vector_delay,
            duty,
            vector_state,
            float_label,
            &samples,
            baseline,
        );

        if health_ok != 1 {
            cycle_ok = 0;
            break;
        }

        vector_steps += 1;
    }

    apply_state(DriveState::IdleAllOff);

    rprintln!(
        "pwm_run_stop run={} cycle_ok={} vector_steps={} button={} reason={}",
        run_id,
        cycle_ok,
        vector_steps,
        if button_pressed() { 1 } else { 0 },
        if cycle_ok == 0 {
            "health_fault"
        } else if vector_steps >= MAX_VECTOR_STEPS_PER_HOLD {
            "max_steps"
        } else {
            "button_released"
        }
    );

    if cycle_ok == 0 {
        apply_state(DriveState::FaultAllOff);
        log_state(run_id, 9998, 0, 0, DriveState::FaultAllOff, baseline);
    }

    log_state(run_id, 9999, 0, 0, DriveState::IdleAllOff, baseline);
}

// ------------------------------------------------------------
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio_base();
    setup_tim1_base();
    setup_drive_pins_tim1_af();
    setup_bemf_pins();

    apply_state(DriveState::IdleAllOff);

    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();

    let baseline = read_adc_snapshot();
    let startup_drive = read_drive();
    let startup_tim1 = read_tim1_for_state(DriveState::IdleAllOff);

    let startup_pins_ok = pins_match_state(startup_drive, DriveState::IdleAllOff);

    let startup_ok = if startup_drive.af_ok == 1
        && startup_pins_ok == 1
        && no_phase_overlap(startup_drive) == 1
        && startup_tim1.tim1_basic_ok == 1
        && baseline.timeout == 0
    {
        1
    } else {
        0
    };

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.4.0-bemf-observe-openloop");
    rprintln!("Mode: PWM open-loop six-step + BEMF observe (loop still OPEN)");
    rprintln!("Button released: all outputs off.");
    rprintln!("Button held: precharge -> align hold -> ramped six-step.");
    rprintln!("Release button to stop immediately.");
    rprintln!("No prop. Low voltage. Strict bench current limit.");
    rprintln!("PWM freq = timer_clk / {} (ARR+1). On HSI16 this is ~20 kHz (inaudible).", TIM1_TEST_ARR + 1);
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);

    rprintln!("BEMF observe (PWM-off sampling, divider disabled):");
    rprintln!("  GPIO_BEMF=PB5 held INPUT/high-Z (divider off, GND ref)");
    rprintln!("  BEMF1=PA4=ADC2_IN17 (U), BEMF2=PC4=ADC2_IN5 (V), BEMF3=PB11=ADC2_IN14 (W)");
    rprintln!("  per step: blank {} then {} samples in PWM-off window", BEMF_BLANK_DELAY, BEMF_SAMPLES_PER_STEP);
    rprintln!("  log line 'bemf ...' shows floating phase b0..b3 per step");
    rprintln!("  VERIFY each phase's BEMF channel responds before trusting data");

    rprintln!("PWM duty (CCR vs ARR={}):", TIM1_TEST_ARR);
    rprintln!("  align duty:      {}", PWM_DUTY_ALIGN);
    rprintln!("  run start duty:  {}", PWM_DUTY_RUN_START);
    rprintln!("  run max duty:    {}", PWM_DUTY_RUN_MAX);
    rprintln!("  inc per erev:    {}", PWM_DUTY_INC_PER_EREV);
    rprintln!("  align hold:      {}", ALIGN_HOLD_DELAY);

    rprintln!("Ramp:");
    rprintln!("  start vector delay: {}", RAMP_START_VECTOR_DELAY);
    rprintln!("  min vector delay:   {}", RAMP_MIN_VECTOR_DELAY);
    rprintln!("  decrement / erev:   {}", RAMP_DECREMENT_PER_ELECTRICAL_REV);
    rprintln!("  max vector steps:   {}", MAX_VECTOR_STEPS_PER_HOLD);

    rprintln!("Six-step sequence (high side PWMs, low side solid on):");
    rprintln!("  1 vector_uh_vl  UH~PWM VL=1");
    rprintln!("  2 vector_uh_wl  UH~PWM WL=1");
    rprintln!("  3 vector_vh_wl  VH~PWM WL=1");
    rprintln!("  4 vector_vh_ul  VH~PWM UL=1");
    rprintln!("  5 vector_wh_ul  WH~PWM UL=1");
    rprintln!("  6 vector_wh_vl  WH~PWM VL=1");

    rprintln!(
        "startup: startup_ok={} af_ok={} pins_ok={} no_phase_overlap={} tim1_ok={} UH={} UL={} VH={} VL={} WH={} WL={} ccer={} moe={} forced_modes_ok={} pot_raw={} temp_raw={} vbus_raw={} op1_raw={} op2_raw={} op3_raw={} timeout={}",
        startup_ok,
        startup_drive.af_ok,
        startup_pins_ok,
        no_phase_overlap(startup_drive),
        startup_tim1.tim1_basic_ok,
        startup_drive.uh_pin,
        startup_drive.ul_pin,
        startup_drive.vh_pin,
        startup_drive.vl_pin,
        startup_drive.wh_pin,
        startup_drive.wl_pin,
        startup_tim1.ccer,
        startup_tim1.moe,
        startup_tim1.forced_modes_ok,
        baseline.pot_raw,
        baseline.temp_raw,
        baseline.vbus_raw,
        baseline.op1_raw,
        baseline.op2_raw,
        baseline.op3_raw,
        baseline.timeout
    );

    rprintln!("Expected idle: startup_ok=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("During run: heartbeat once per erev; high pins flicker (PWM).");
    rprintln!("Safety stops: button release, health fault (af/overlap/temp/timeout), max step count.");
    rprintln!("================================================");

    let mut run_id: u32 = 1;

    loop {
        apply_state(DriveState::IdleAllOff);

        if button_pressed() {
            run_pwm_openloop_ramp(run_id, baseline);

            while button_pressed() {
                apply_state(DriveState::IdleAllOff);
                delay_cycles(RELEASE_CHECK_CHUNK);
            }

            run_id = run_id.wrapping_add(1);
        } else {
            led_high();
            delay_cycles(IDLE_LED_ON_DELAY);
            led_low();

            log_state(run_id, 0, 0, 0, DriveState::IdleAllOff, baseline);

            delay_cycles(IDLE_LED_OFF_DELAY);
            delay_cycles(IDLE_LOG_DELAY);
        }
    }
}

// ================================================================
// Footer
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.4.0-bemf-observe-openloop
// Created: 2026-06-07
// Generated timestamp: 2026-06-07T00:00:00Z
// ================================================================