// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.2.3-fix1-tim1-ccer-enabled-moe-off
// Purpose: STM32G431CB Rust bring-up: TIM1 AF drive pins with CCER enabled but BDTR.MOE kept OFF
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
const TIM1_BDTR_MOE: u32 = 1 << 15;

// CCER enable bits only. Polarity bits remain zero, active-high.
// Decimal expected value: 1365.
const TIM1_CCER_SAFE_ENABLED_MOE_OFF: u32 =
    (1 << 0) |  // CC1E
    (1 << 2) |  // CC1NE
    (1 << 4) |  // CC2E
    (1 << 6) |  // CC2NE
    (1 << 8) |  // CC3E
    (1 << 10);  // CC3NE

const TIM1_TEST_PSC: u32 = 0;
const TIM1_TEST_ARR: u32 = 3999;

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

const STATUS_LED_PIN: u32 = 6;   // PC6
const USER_BUTTON_PIN: u32 = 10; // PC10

const VBUS_PIN: u32 = 0;    // PA0
const OP1_OUT_PIN: u32 = 2; // PA2
const OP2_OUT_PIN: u32 = 6; // PA6

const OP3_OUT_PIN: u32 = 1; // PB1
const POT_PIN: u32 = 12;    // PB12
const TEMP_PIN: u32 = 14;   // PB14

// L6387 drive input pins.
const DRIVE_UH_PIN: u32 = 8;  // PA8  / TIM1_CH1  / UH
const DRIVE_VH_PIN: u32 = 9;  // PA9  / TIM1_CH2  / VH
const DRIVE_WH_PIN: u32 = 10; // PA10 / TIM1_CH3  / WH
const DRIVE_VL_PIN: u32 = 12; // PA12 / TIM1_CH2N / VL
const DRIVE_WL_PIN: u32 = 15; // PB15 / TIM1_CH3N / WL
const DRIVE_UL_PIN: u32 = 13; // PC13 / TIM1_CH1N / UL

// TIM1 alternate-function numbers.
const AF_TIM1_PA: u32 = 6; // PA8/PA9/PA10/PA12
const AF_TIM1_N: u32 = 4;  // PB15/PC13

// ADC channels.
const VBUS_ADC_CHANNEL: u32 = 1;     // PA0  / ADC1_IN1
const OP1_OUT_ADC_CHANNEL: u32 = 3;  // PA2  / ADC1_IN3
const OP2_OUT_ADC_CHANNEL: u32 = 3;  // PA6  / ADC2_IN3
const OP3_OUT_ADC_CHANNEL: u32 = 12; // PB1  / ADC1_IN12
const TEMP_ADC_CHANNEL: u32 = 5;     // PB14 / ADC1_IN5
const POT_ADC_CHANNEL: u32 = 11;     // PB12 / ADC1_IN11

const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

// ------------------------------------------------------------
// Readback structures
// ------------------------------------------------------------

#[derive(Copy, Clone)]
struct DriveAfReadback {
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
    psc: u32,
    arr: u32,
    ccr1: u32,
    ccr2: u32,
    ccr3: u32,
    ccer: u32,
    bdtr: u32,
    cr2: u32,
    moe: u32,
    ossi: u32,
    ccer_expected: u32,
    safety_ok: u32,
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

fn read_drive_af() -> DriveAfReadback {
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

    DriveAfReadback {
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

// ------------------------------------------------------------
// Delay helpers
// ------------------------------------------------------------

fn delay_cycles(cycles: u32) {
    asm::delay(cycles);
}

fn delay_fast_button() {
    asm::delay(650_000);
}

// ------------------------------------------------------------
// TIM1 helpers
// ------------------------------------------------------------

fn enforce_tim1_ccer_enabled_moe_off() {
    unsafe {
        // No duty yet.
        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        // Enable CC outputs internally, but keep the advanced-timer master gate OFF.
        write_volatile(TIM1_CCER, TIM1_CCER_SAFE_ENABLED_MOE_OFF);

        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr &= !TIM1_BDTR_MOE;
        bdtr |= TIM1_BDTR_OSSI;
        write_volatile(TIM1_BDTR, bdtr);
    }
}

fn read_tim1() -> Tim1Readback {
    unsafe {
        let cnt_a = read_volatile(TIM1_CNT);
        asm::delay(16_000);
        let cnt_b = read_volatile(TIM1_CNT);

        let psc = read_volatile(TIM1_PSC);
        let arr = read_volatile(TIM1_ARR);
        let ccr1 = read_volatile(TIM1_CCR1);
        let ccr2 = read_volatile(TIM1_CCR2);
        let ccr3 = read_volatile(TIM1_CCR3);
        let ccer = read_volatile(TIM1_CCER);
        let bdtr = read_volatile(TIM1_BDTR);
        let cr2 = read_volatile(TIM1_CR2);

        let counting = if cnt_a != cnt_b { 1 } else { 0 };
        let moe = if (bdtr & TIM1_BDTR_MOE) != 0 { 1 } else { 0 };
        let ossi = if (bdtr & TIM1_BDTR_OSSI) != 0 { 1 } else { 0 };

        let ccer_expected = if ccer == TIM1_CCER_SAFE_ENABLED_MOE_OFF {
            1
        } else {
            0
        };

        // Safety definition for this step:
        // - TIM1 is counting.
        // - CCER has channel output enables set.
        // - MOE remains OFF.
        // - CCRs remain zero.
        // - OSSI is set for defined idle state while MOE is off.
        // - CR2 idle state bits remain zero.
        let idle_bits = (cr2 >> 8) & 0b11_1111;

        let safety_ok = if counting == 1
            && ccer_expected == 1
            && moe == 0
            && ossi == 1
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
            psc,
            arr,
            ccr1,
            ccr2,
            ccr3,
            ccer,
            bdtr,
            cr2,
            moe,
            ossi,
            ccer_expected,
            safety_ok,
        }
    }
}

fn setup_tim1_ccer_enabled_moe_off() {
    unsafe {
        let apb2enr = read_volatile(RCC_APB2ENR);
        write_volatile(RCC_APB2ENR, apb2enr | RCC_APB2ENR_TIM1EN);

        asm::delay(8_000);

        // Stop timer while configuring.
        write_volatile(TIM1_CR1, 0);

        // Start from fully disabled output state.
        write_volatile(TIM1_CCER, 0);

        // CR2 idle states: OIS1/OIS1N/OIS2/OIS2N/OIS3/OIS3N = 0.
        // This requests LOW idle state for all six outputs.
        let mut cr2 = read_volatile(TIM1_CR2);
        cr2 &= !(0b11_1111 << 8);
        write_volatile(TIM1_CR2, cr2);

        // MOE remains OFF.
        // OSSI ON gives a defined idle state while MOE is off.
        let mut bdtr = read_volatile(TIM1_BDTR);
        bdtr &= !TIM1_BDTR_MOE;
        bdtr |= TIM1_BDTR_OSSI;
        write_volatile(TIM1_BDTR, bdtr);

        write_volatile(TIM1_PSC, TIM1_TEST_PSC);
        write_volatile(TIM1_ARR, TIM1_TEST_ARR);
        write_volatile(TIM1_RCR, 0);

        // No duty yet.
        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        // PWM mode 1 internally for CH1/CH2/CH3, preload enabled.
        let ccmr1 = (0b110 << 4) | (1 << 3) | (0b110 << 12) | (1 << 11);
        let ccmr2 = (0b110 << 4) | (1 << 3);

        write_volatile(TIM1_CCMR1, ccmr1);
        write_volatile(TIM1_CCMR2, ccmr2);

        // Update event to load registers.
        write_volatile(TIM1_EGR, TIM1_EGR_UG);

        enforce_tim1_ccer_enabled_moe_off();

        // Start counter.
        write_volatile(TIM1_CR1, TIM1_CR1_ARPE | TIM1_CR1_CEN);

        enforce_tim1_ccer_enabled_moe_off();
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

fn adc_live_percent(raw: u16) -> u32 {
    let clamped_raw = if raw > 4095 { 2048 } else { raw };
    ((clamped_raw as u32) * 100) / 4095
}

fn adc_delta(current: u16, baseline: u16) -> i32 {
    (current as i32) - (baseline as i32)
}

fn pot_to_delay(raw: u16) -> u32 {
    let clamped_raw = if raw > 4095 { 2048 } else { raw };
    let raw_u64 = clamped_raw as u64;

    let fast_delay: u64 = 650_000;
    let slow_delay: u64 = 4_000_000;
    let span: u64 = slow_delay - fast_delay;

    let delay = slow_delay - ((raw_u64 * span) / 4095);

    delay as u32
}

fn blink_with_delay(delay: u32) {
    led_high();
    delay_cycles(delay);

    led_low();
    delay_cycles(delay);
}

fn blink_fast_button_override() {
    led_high();
    delay_fast_button();

    led_low();
    delay_fast_button();
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
        // Make sure TIM1 is safe before switching pins to AF.
        write_volatile(TIM1_CCER, 0);
        write_volatile(TIM1_BDTR, (read_volatile(TIM1_BDTR) & !TIM1_BDTR_MOE) | TIM1_BDTR_OSSI);
        write_volatile(TIM1_CCR1, 0);
        write_volatile(TIM1_CCR2, 0);
        write_volatile(TIM1_CCR3, 0);

        force_drive_output_latches_low();

        // GPIOA TIM1 AF pins: PA8/PA9/PA10/PA12 = AF6.
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

        // GPIOB TIM1 AF pin: PB15 = AF4.
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

        // GPIOC TIM1 AF pin: PC13 = AF4.
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

        // FIX1:
        // The original v0.2.3 cleared CCER here and left it cleared.
        // This corrected version re-applies the intended TIM1 state after AF setup.
        enforce_tim1_ccer_enabled_moe_off();
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
// Main
// ------------------------------------------------------------

#[entry]
fn main() -> ! {
    rtt_init_print!();

    setup_gpio_base();
    setup_tim1_ccer_enabled_moe_off();
    setup_drive_pins_tim1_af();

    // Final safety enforcement after all pin/timer setup.
    enforce_tim1_ccer_enabled_moe_off();

    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();

    let drive_startup = read_drive_af();
    let tim1_startup = read_tim1();

    let temp_startup_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_startup_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let op1_startup_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    let op2_startup_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    let op3_startup_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.2.3-fix1-tim1-ccer-enabled-moe-off");
    rprintln!("Drive pins are TIM1 alternate function.");
    rprintln!("CCER enables CH1/1N/2/2N/3/3N, but BDTR.MOE remains 0.");
    rprintln!("CCR1/2/3 remain 0. Idle state bits remain 0. OSSI is set.");
    rprintln!("No intentional gate switching in this version.");
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);

    rprintln!(
        "drive_startup: af_ok={} UH_pin={} UL_pin={} VH_pin={} VL_pin={} WH_pin={} WL_pin={} UH_mode={} UL_mode={} VH_mode={} VL_mode={} WH_mode={} WL_mode={} UH_af={} UL_af={} VH_af={} VL_af={} WH_af={} WL_af={}",
        drive_startup.af_ok,
        drive_startup.uh_pin,
        drive_startup.ul_pin,
        drive_startup.vh_pin,
        drive_startup.vl_pin,
        drive_startup.wh_pin,
        drive_startup.wl_pin,
        drive_startup.uh_mode,
        drive_startup.ul_mode,
        drive_startup.vh_mode,
        drive_startup.vl_mode,
        drive_startup.wh_mode,
        drive_startup.wl_mode,
        drive_startup.uh_af,
        drive_startup.ul_af,
        drive_startup.vh_af,
        drive_startup.vl_af,
        drive_startup.wh_af,
        drive_startup.wl_af
    );

    rprintln!(
        "tim1_startup: safety_ok={} counting={} cnt_a={} cnt_b={} psc={} arr={} ccr1={} ccr2={} ccr3={} ccer={} ccer_expected={} bdtr={} cr2={} moe={} ossi={}",
        tim1_startup.safety_ok,
        tim1_startup.counting,
        tim1_startup.cnt_a,
        tim1_startup.cnt_b,
        tim1_startup.psc,
        tim1_startup.arr,
        tim1_startup.ccr1,
        tim1_startup.ccr2,
        tim1_startup.ccr3,
        tim1_startup.ccer,
        tim1_startup.ccer_expected,
        tim1_startup.bdtr,
        tim1_startup.cr2,
        tim1_startup.moe,
        tim1_startup.ossi
    );

    rprintln!("temp_startup_raw: {}", temp_startup_raw);
    rprintln!("vbus_startup_raw: {}", vbus_startup_raw);
    rprintln!("op1_startup_raw: {}", op1_startup_raw);
    rprintln!("op2_startup_raw: {}", op2_startup_raw);
    rprintln!("op3_startup_raw: {}", op3_startup_raw);
    rprintln!("Output format:");
    rprintln!("button=<0/1> safety_ok=<0/1> af_ok=<0/1> tim1_counting=<0/1> tim1_ccer=<raw> tim1_moe=<0/1> tim1_ossi=<0/1> UH_pin=<0/1> UL_pin=<0/1> VH_pin=<0/1> VL_pin=<0/1> WH_pin=<0/1> WL_pin=<0/1> pot_raw=<raw> temp_raw=<raw> vbus_raw=<raw> op1_raw=<raw> op2_raw=<raw> op3_raw=<raw> timeout=<0/1>");
    rprintln!("================================================");

    led_low();

    loop {
        // Keep the intended safety state latched while monitoring.
        enforce_tim1_ccer_enabled_moe_off();

        let drive = read_drive_af();
        let tim1 = read_tim1();

        let pot_raw = adc_read_channel_raw(ADC1_BASE, POT_ADC_CHANNEL);
        let temp_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
        let vbus_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

        let op1_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
        let op2_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
        let op3_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

        let pot_timeout = pot_raw == ADC_TIMEOUT_VALUE;
        let temp_timeout = temp_raw == ADC_TIMEOUT_VALUE;
        let vbus_timeout = vbus_raw == ADC_TIMEOUT_VALUE;
        let op1_timeout = op1_raw == ADC_TIMEOUT_VALUE;
        let op2_timeout = op2_raw == ADC_TIMEOUT_VALUE;
        let op3_timeout = op3_raw == ADC_TIMEOUT_VALUE;

        let timeout = pot_timeout
            || temp_timeout
            || vbus_timeout
            || op1_timeout
            || op2_timeout
            || op3_timeout;

        let live_pot_raw = if pot_timeout { 2048 } else { pot_raw };
        let live_temp_raw = if temp_timeout { temp_startup_raw } else { temp_raw };
        let live_vbus_raw = if vbus_timeout { vbus_startup_raw } else { vbus_raw };

        let live_op1_raw = if op1_timeout { op1_startup_raw } else { op1_raw };
        let live_op2_raw = if op2_timeout { op2_startup_raw } else { op2_raw };
        let live_op3_raw = if op3_timeout { op3_startup_raw } else { op3_raw };

        let pot_pct = adc_live_percent(live_pot_raw);
        let temp_delta = adc_delta(live_temp_raw, temp_startup_raw);
        let vbus_delta = adc_delta(live_vbus_raw, vbus_startup_raw);

        let op1_delta = adc_delta(live_op1_raw, op1_startup_raw);
        let op2_delta = adc_delta(live_op2_raw, op2_startup_raw);
        let op3_delta = adc_delta(live_op3_raw, op3_startup_raw);

        let delay = pot_to_delay(live_pot_raw);

        if button_pressed() {
            rprintln!(
                "button=1 safety_ok={} af_ok={} tim1_counting={} tim1_cnt_a={} tim1_cnt_b={} tim1_arr={} tim1_ccr1={} tim1_ccr2={} tim1_ccr3={} tim1_ccer={} tim1_ccer_expected={} tim1_bdtr={} tim1_cr2={} tim1_moe={} tim1_ossi={} UH_pin={} UL_pin={} VH_pin={} VL_pin={} WH_pin={} WL_pin={} UH_af={} UL_af={} VH_af={} VL_af={} WH_af={} WL_af={} pot_raw={} pot_pct={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} delay_cycles={} timeout={} mode=button_fast",
                tim1.safety_ok,
                drive.af_ok,
                tim1.counting,
                tim1.cnt_a,
                tim1.cnt_b,
                tim1.arr,
                tim1.ccr1,
                tim1.ccr2,
                tim1.ccr3,
                tim1.ccer,
                tim1.ccer_expected,
                tim1.bdtr,
                tim1.cr2,
                tim1.moe,
                tim1.ossi,
                drive.uh_pin,
                drive.ul_pin,
                drive.vh_pin,
                drive.vl_pin,
                drive.wh_pin,
                drive.wl_pin,
                drive.uh_af,
                drive.ul_af,
                drive.vh_af,
                drive.vl_af,
                drive.wh_af,
                drive.wl_af,
                live_pot_raw,
                pot_pct,
                live_temp_raw,
                temp_delta,
                live_vbus_raw,
                vbus_delta,
                live_op1_raw,
                op1_delta,
                live_op2_raw,
                op2_delta,
                live_op3_raw,
                op3_delta,
                650_000,
                if timeout { 1 } else { 0 }
            );

            blink_fast_button_override();
        } else {
            rprintln!(
                "button=0 safety_ok={} af_ok={} tim1_counting={} tim1_cnt_a={} tim1_cnt_b={} tim1_arr={} tim1_ccr1={} tim1_ccr2={} tim1_ccr3={} tim1_ccer={} tim1_ccer_expected={} tim1_bdtr={} tim1_cr2={} tim1_moe={} tim1_ossi={} UH_pin={} UL_pin={} VH_pin={} VL_pin={} WH_pin={} WL_pin={} UH_af={} UL_af={} VH_af={} VL_af={} WH_af={} WL_af={} pot_raw={} pot_pct={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} delay_cycles={} timeout={} mode=pot_control",
                tim1.safety_ok,
                drive.af_ok,
                tim1.counting,
                tim1.cnt_a,
                tim1.cnt_b,
                tim1.arr,
                tim1.ccr1,
                tim1.ccr2,
                tim1.ccr3,
                tim1.ccer,
                tim1.ccer_expected,
                tim1.bdtr,
                tim1.cr2,
                tim1.moe,
                tim1.ossi,
                drive.uh_pin,
                drive.ul_pin,
                drive.vh_pin,
                drive.vl_pin,
                drive.wh_pin,
                drive.wl_pin,
                drive.uh_af,
                drive.ul_af,
                drive.vh_af,
                drive.vl_af,
                drive.wh_af,
                drive.wl_af,
                live_pot_raw,
                pot_pct,
                live_temp_raw,
                temp_delta,
                live_vbus_raw,
                vbus_delta,
                live_op1_raw,
                op1_delta,
                live_op2_raw,
                op2_delta,
                live_op3_raw,
                op3_delta,
                delay,
                if timeout { 1 } else { 0 }
            );

            blink_with_delay(delay);
        }
    }
}

// ================================================================
// Footer
// File: main.rs
// Version: v0.2.3-fix1-tim1-ccer-enabled-moe-off
// Created: 2026-06-07
// Generated timestamp: 2026-06-07
// ================================================================