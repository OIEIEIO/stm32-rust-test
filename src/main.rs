// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.2.14-button-gated-u-twitch-prep
// Purpose: STM32G431CB Rust bring-up: button-gated U->V motor twitch prep, no motor first
// Target: B-G431B-ESC1, STM32G431CB, Cortex-M4F
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

// All-off forced-low state from v0.2.5-fix1.
// Expected: UH=0 UL=0 VH=0 VL=0 WH=0 WL=0.
// Decimal: 3549.
const TIM1_CCER_ALL_OFF_FORCED_LOW: u32 =
    (1 << 0) | (1 << 2) | (1 << 3) |
    (1 << 4) | (1 << 6) | (1 << 7) |
    (1 << 8) | (1 << 10) | (1 << 11);

// U bootstrap charge.
// Expected: UH=0 UL=1 VH=0 VL=0 WH=0 WL=0.
// Decimal: 5.
const TIM1_CCER_U_BOOTSTRAP_UL: u32 =
    (1 << 0) |
    (1 << 2);

// First twitch vector.
// Expected: UH=1 UL=0 VH=0 VL=1 WH=0 WL=0.
// Decimal: 81.
const TIM1_CCER_TWITCH_UH_VL: u32 =
    (1 << 0) |
    (1 << 4) |
    (1 << 6);

const TIM1_TEST_PSC: u32 = 0;
const TIM1_TEST_ARR: u32 = 3999;

// Timings are intentionally short and conservative.
// At ~170 MHz CPU clock, these are only approximate.
const IDLE_LOG_DELAY: u32 = 6_000_000;
const STATE_SETTLE_DELAY: u32 = 250_000;
const BOOTSTRAP_HOLD_DELAY: u32 = 500_000;
const DEADTIME_DELAY: u32 = 250_000;
const TWITCH_HOLD_DELAY: u32 = 500_000;
const AFTER_TWITCH_DELAY: u32 = 2_000_000;
const WAIT_RELEASE_DELAY: u32 = 3_000_000;

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
// State definitions
// ------------------------------------------------------------

#[derive(Copy, Clone)]
enum TwitchState {
    IdleAllOff,
    UBootstrapChargeUl,
    DeadtimeAfterUl,
    TwitchDriveUhVl,
    AllOffAfterTwitch,
    WaitButtonRelease,
}

impl TwitchState {
    fn name(self) -> &'static str {
        match self {
            TwitchState::IdleAllOff => "idle_all_off",
            TwitchState::UBootstrapChargeUl => "u_bootstrap_charge_ul",
            TwitchState::DeadtimeAfterUl => "deadtime_after_ul",
            TwitchState::TwitchDriveUhVl => "twitch_drive_uh_vl",
            TwitchState::AllOffAfterTwitch => "all_off_after_twitch",
            TwitchState::WaitButtonRelease => "wait_button_release_all_off",
        }
    }

    fn expected_ccer(self) -> u32 {
        match self {
            TwitchState::IdleAllOff => TIM1_CCER_ALL_OFF_FORCED_LOW,
            TwitchState::UBootstrapChargeUl => TIM1_CCER_U_BOOTSTRAP_UL,
            TwitchState::DeadtimeAfterUl => TIM1_CCER_ALL_OFF_FORCED_LOW,
            TwitchState::TwitchDriveUhVl => TIM1_CCER_TWITCH_UH_VL,
            TwitchState::AllOffAfterTwitch => TIM1_CCER_ALL_OFF_FORCED_LOW,
            TwitchState::WaitButtonRelease => TIM1_CCER_ALL_OFF_FORCED_LOW,
        }
    }

    fn expected_pins(self) -> (u32, u32, u32, u32, u32, u32) {
        match self {
            TwitchState::IdleAllOff => (0, 0, 0, 0, 0, 0),
            TwitchState::UBootstrapChargeUl => (0, 1, 0, 0, 0, 0),
            TwitchState::DeadtimeAfterUl => (0, 0, 0, 0, 0, 0),
            TwitchState::TwitchDriveUhVl => (1, 0, 0, 1, 0, 0),
            TwitchState::AllOffAfterTwitch => (0, 0, 0, 0, 0, 0),
            TwitchState::WaitButtonRelease => (0, 0, 0, 0, 0, 0),
        }
    }

    fn ch1_mode(self) -> u32 {
        match self {
            TwitchState::TwitchDriveUhVl => TIM1_CCMR_FORCED_ACTIVE,
            _ => TIM1_CCMR_FORCED_INACTIVE,
        }
    }

    fn ch2_mode(self) -> u32 {
        TIM1_CCMR_FORCED_INACTIVE
    }

    fn ch3_mode(self) -> u32 {
        TIM1_CCMR_FORCED_INACTIVE
    }

    fn active_led(self) -> bool {
        match self {
            TwitchState::UBootstrapChargeUl | TwitchState::TwitchDriveUhVl => true,
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

    uh_mode: u32,
    ul_mode: u32,
    vh_mode: u32,
    vl_mode: u32,
    wh_mode: u32,
    wl_mode: u32,

    uh_af: u32,
    ul_af: u32,
    vh_af: u32,
    vl_af: u32,
    wh_af: u32,
    wl_af: u32,

    af_ok: u32,
}

#[derive(Copy, Clone)]
struct Tim1Readback {
    cnt_a: u32,
    cnt_b: u32,
    counting: u32,
    ccer: u32,
    bdtr: u32,
    ccmr1: u32,
    ccmr2: u32,
    moe: u32,
    ossi: u32,
    ossr: u32,
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

fn led_for_state(state: TwitchState) {
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
        uh_mode,
        ul_mode,
        vh_mode,
        vl_mode,
        wh_mode,
        wl_mode,
        uh_af,
        ul_af,
        vh_af,
        vl_af,
        wh_af,
        wl_af,
        af_ok,
    }
}

fn pins_match_state(drive: DriveReadback, state: TwitchState) -> u32 {
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
// Delay helper
// ------------------------------------------------------------

fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

// ------------------------------------------------------------
// TIM1 helpers
// ------------------------------------------------------------

fn set_tim1_modes_for_state(state: TwitchState) {
    unsafe {
        let ccmr1 =
            (state.ch1_mode() << 4) |
            (state.ch2_mode() << 12);

        let ccmr2 = state.ch3_mode() << 4;

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);
    }
}

fn apply_state(state: TwitchState) {
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

fn read_tim1_for_state(state: TwitchState) -> Tim1Readback {
    unsafe {
        let cnt_a = read_volatile(TIM1_CNT);
        asm::delay(16_000);
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
            counting,
            ccer,
            bdtr,
            ccmr1,
            ccmr2,
            moe,
            ossi,
            ossr,
            forced_modes_ok,
            tim1_basic_ok,
        }
    }
}

fn setup_tim1_base() {
    unsafe {
        let apb2enr = read_volatile(RCC_APB2ENR);
        write_volatile(RCC_APB2ENR, apb2enr | RCC_APB2ENR_TIM1EN);

        asm::delay(8_000);

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

        set_tim1_modes_for_state(TwitchState::IdleAllOff);

        write_volatile(TIM1_EGR, TIM1_EGR_UG);
        write_volatile(TIM1_CR1, TIM1_CR1_ARPE | TIM1_CR1_CEN);

        apply_state(TwitchState::IdleAllOff);
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

        asm::delay(8_000);

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

        apply_state(TwitchState::IdleAllOff);
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

        asm::delay(160_000);

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
// Logging / sequence
// ------------------------------------------------------------

fn log_state(
    cycle: u32,
    step: u32,
    state: TwitchState,
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
    let expected_active_count = expected.0 + expected.1 + expected.2 + expected.3 + expected.4 + expected.5;

    let active_count_ok = if active_count == expected_active_count {
        1
    } else {
        0
    };

    let state_ok = if drive.af_ok == 1
        && pins_ok == 1
        && no_overlap == 1
        && active_count_ok == 1
        && tim1.tim1_basic_ok == 1
        && adc.timeout == 0
    {
        1
    } else {
        0
    };

    rprintln!(
        "cycle={} step={} state={} state_ok={} button={} af_ok={} pins_ok={} no_phase_overlap={} active_count={} active_count_ok={} tim1_ok={} expected_ccer={} tim1_ccer={} tim1_moe={} forced_modes_ok={} UH={} UL={} VH={} VL={} WH={} WL={} exp_UH={} exp_UL={} exp_VH={} exp_VL={} exp_WH={} exp_WL={} cnt_a={} cnt_b={} ccmr1={} ccmr2={} bdtr={} pot_raw={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} timeout={}",
        cycle,
        step,
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
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        expected.4,
        expected.5,
        tim1.cnt_a,
        tim1.cnt_b,
        tim1.ccmr1,
        tim1.ccmr2,
        tim1.bdtr,
        adc.pot_raw,
        adc.temp_raw,
        adc_delta(adc.temp_raw, baseline.temp_raw),
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

fn run_state(
    cycle: u32,
    step: u32,
    state: TwitchState,
    hold_delay: u32,
    baseline: AdcSnapshot,
) -> u32 {
    apply_state(state);
    let state_ok = log_state(cycle, step, state, baseline);
    delay_cycles(hold_delay);
    state_ok
}

fn run_single_twitch(cycle: u32, baseline: AdcSnapshot) -> u32 {
    let mut cycle_ok = 1;

    cycle_ok &= run_state(
        cycle,
        1,
        TwitchState::IdleAllOff,
        DEADTIME_DELAY,
        baseline,
    );

    cycle_ok &= run_state(
        cycle,
        2,
        TwitchState::UBootstrapChargeUl,
        BOOTSTRAP_HOLD_DELAY,
        baseline,
    );

    cycle_ok &= run_state(
        cycle,
        3,
        TwitchState::DeadtimeAfterUl,
        DEADTIME_DELAY,
        baseline,
    );

    cycle_ok &= run_state(
        cycle,
        4,
        TwitchState::TwitchDriveUhVl,
        TWITCH_HOLD_DELAY,
        baseline,
    );

    cycle_ok &= run_state(
        cycle,
        5,
        TwitchState::AllOffAfterTwitch,
        AFTER_TWITCH_DELAY,
        baseline,
    );

    apply_state(TwitchState::IdleAllOff);

    rprintln!(
        "twitch_summary cycle={} cycle_ok={} twitch_vector=UH_plus_VL motor_test_note=no_prop_low_voltage_current_limit",
        cycle,
        cycle_ok
    );

    cycle_ok
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

    apply_state(TwitchState::IdleAllOff);

    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();

    let baseline = read_adc_snapshot();
    let startup_drive = read_drive();
    let startup_tim1 = read_tim1_for_state(TwitchState::IdleAllOff);

    let startup_pins_ok = pins_match_state(startup_drive, TwitchState::IdleAllOff);

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
    rprintln!("Version: v0.2.14-button-gated-u-twitch-prep");
    rprintln!("Mode: button-gated single U->V twitch prep");
    rprintln!("Default state is all outputs off.");
    rprintln!("Button press runs one short sequence:");
    rprintln!("  idle_all_off            UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("  u_bootstrap_charge_ul   UH=0 UL=1 VH=0 VL=0 WH=0 WL=0");
    rprintln!("  deadtime_after_ul       UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("  twitch_drive_uh_vl      UH=1 UL=0 VH=0 VL=1 WH=0 WL=0");
    rprintln!("  all_off_after_twitch    UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("Safety: test no motor first.");
    rprintln!("Motor test later: no prop, low voltage, strict current limit, short button tap only.");
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);

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

    rprintln!("Expected idle: state_ok=1 UH=0 UL=0 VH=0 VL=0 WH=0 WL=0");
    rprintln!("Expected twitch state: state=twitch_drive_uh_vl state_ok=1 UH=1 VL=1 active_count=2 no_phase_overlap=1");
    rprintln!("================================================");

    let mut cycle: u32 = 1;

    loop {
        if button_pressed() {
            let cycle_ok = run_single_twitch(cycle, baseline);

            apply_state(TwitchState::WaitButtonRelease);
            log_state(cycle, 6, TwitchState::WaitButtonRelease, baseline);

            while button_pressed() {
                apply_state(TwitchState::WaitButtonRelease);
                delay_cycles(WAIT_RELEASE_DELAY);
            }

            rprintln!(
                "button_release cycle={} prior_twitch_ok={} outputs=all_off",
                cycle,
                cycle_ok
            );

            cycle = cycle.wrapping_add(1);
        } else {
            apply_state(TwitchState::IdleAllOff);
            log_state(cycle, 0, TwitchState::IdleAllOff, baseline);
            delay_cycles(IDLE_LOG_DELAY);
        }
    }
}

// ================================================================
// Footer
// File: main.rs
// Version: v0.2.14-button-gated-u-twitch-prep
// Created: 2026-06-07
// Generated timestamp: 2026-06-07
// ================================================================