// ================================================================
// File: main.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/src/main.rs
// Version: v0.1.9-opamp-output-raw-monitor
// Purpose: STM32G431CB Rust bring-up: live pot/temp/VBUS + raw OPAMP output ADC monitor over RTT
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

// ------------------------------------------------------------
// RCC registers
// ------------------------------------------------------------

const RCC_AHB2ENR: *mut u32 = (RCC_BASE + 0x4C) as *mut u32;

const RCC_AHB2ENR_GPIOAEN: u32 = 1 << 0;
const RCC_AHB2ENR_GPIOBEN: u32 = 1 << 1;
const RCC_AHB2ENR_GPIOCEN: u32 = 1 << 2;
const RCC_AHB2ENR_ADC12EN: u32 = 1 << 13;

// ------------------------------------------------------------
// GPIO registers
// ------------------------------------------------------------

const GPIOA_MODER: *mut u32 = (GPIOA_BASE + 0x00) as *mut u32;
const GPIOA_PUPDR: *mut u32 = (GPIOA_BASE + 0x0C) as *mut u32;
const GPIOA_ASCR: *mut u32 = (GPIOA_BASE + 0x2C) as *mut u32;

const GPIOB_MODER: *mut u32 = (GPIOB_BASE + 0x00) as *mut u32;
const GPIOB_PUPDR: *mut u32 = (GPIOB_BASE + 0x0C) as *mut u32;
const GPIOB_ASCR: *mut u32 = (GPIOB_BASE + 0x2C) as *mut u32;

const GPIOC_MODER: *mut u32 = (GPIOC_BASE + 0x00) as *mut u32;
const GPIOC_PUPDR: *mut u32 = (GPIOC_BASE + 0x0C) as *mut u32;
const GPIOC_IDR: *const u32 = (GPIOC_BASE + 0x10) as *const u32;
const GPIOC_BSRR: *mut u32 = (GPIOC_BASE + 0x18) as *mut u32;

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

// ADC ISR bits
const ADC_ISR_ADRDY: u32 = 1 << 0;
const ADC_ISR_EOC: u32 = 1 << 2;
const ADC_ISR_EOS: u32 = 1 << 3;

// ADC CR bits
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

const VBUS_PIN: u32 = 0; // PA0
const OP1_OUT_PIN: u32 = 2; // PA2
const OP2_OUT_PIN: u32 = 6; // PA6

const OP3_OUT_PIN: u32 = 1; // PB1
const POT_PIN: u32 = 12; // PB12
const TEMP_PIN: u32 = 14; // PB14

// ADC channels
const VBUS_ADC_CHANNEL: u32 = 1; // PA0 / ADC1_IN1
const OP1_OUT_ADC_CHANNEL: u32 = 3; // PA2 / ADC1_IN3
const OP2_OUT_ADC_CHANNEL: u32 = 3; // PA6 / ADC2_IN3
const OP3_OUT_ADC_CHANNEL: u32 = 12; // PB1 / ADC1_IN12
const TEMP_ADC_CHANNEL: u32 = 5; // PB14 / ADC1_IN5
const POT_ADC_CHANNEL: u32 = 11; // PB12 / ADC1_IN11

const ADC_TIMEOUT_VALUE: u16 = 0xFFFF;

// ------------------------------------------------------------
// GPIO helpers
// ------------------------------------------------------------

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

    // Confirmed from v0.1.2:
    // PC10 button is active-low with pull-up.
    (idr & (1 << USER_BUTTON_PIN)) == 0
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
// ADC helpers
// ------------------------------------------------------------

fn adc1_select_channel(adc_base: usize, channel: u32) {
    unsafe {
        // Regular sequence length = 1 conversion.
        // SQ1 = selected channel, bits [10:6].
        let sqr1 = channel << 6;
        write_volatile(adc_sqr1(adc_base), sqr1);
    }
}

fn adc_set_sample_time(adc_base: usize, channel: u32) {
    unsafe {
        // Use longest sampling time for all bring-up channels.
        // Good for high-impedance dividers / sensor networks.
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
        adc1_select_channel(adc_base, channel);

        // Clear old EOC/EOS flags.
        write_volatile(adc_isr(adc_base), ADC_ISR_EOC | ADC_ISR_EOS);

        // Start regular conversion.
        let cr = read_volatile(adc_cr(adc_base));
        write_volatile(adc_cr(adc_base), cr | ADC_CR_ADSTART);

        // Poll for conversion complete with a timeout.
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
    // Pot behavior observed:
    // far right -> low / near zero
    // far left  -> high / near 4095
    //
    // low raw  -> slow blink
    // high raw -> fast blink
    //
    // Uses u64 to avoid release-mode u32 overflow.

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

fn setup_gpio() {
    unsafe {
        // Enable GPIOA, GPIOB, GPIOC, and ADC12 clocks.
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

        // GPIOA analog pins:
        // PA0 = VBUS
        // PA2 = OP1_OUT
        // PA6 = OP2_OUT
        let mut gpioa_moder = read_volatile(GPIOA_MODER);

        gpioa_moder &= !(0b11 << (VBUS_PIN * 2));
        gpioa_moder |= 0b11 << (VBUS_PIN * 2);

        gpioa_moder &= !(0b11 << (OP1_OUT_PIN * 2));
        gpioa_moder |= 0b11 << (OP1_OUT_PIN * 2);

        gpioa_moder &= !(0b11 << (OP2_OUT_PIN * 2));
        gpioa_moder |= 0b11 << (OP2_OUT_PIN * 2);

        write_volatile(GPIOA_MODER, gpioa_moder);

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

        // GPIOB analog pins:
        // PB1  = OP3_OUT
        // PB12 = potentiometer
        // PB14 = temperature feedback
        let mut gpiob_moder = read_volatile(GPIOB_MODER);

        gpiob_moder &= !(0b11 << (OP3_OUT_PIN * 2));
        gpiob_moder |= 0b11 << (OP3_OUT_PIN * 2);

        gpiob_moder &= !(0b11 << (POT_PIN * 2));
        gpiob_moder |= 0b11 << (POT_PIN * 2);

        gpiob_moder &= !(0b11 << (TEMP_PIN * 2));
        gpiob_moder |= 0b11 << (TEMP_PIN * 2);

        write_volatile(GPIOB_MODER, gpiob_moder);

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

        // PC6 = output for STATUS LED.
        let mut gpioc_moder = read_volatile(GPIOC_MODER);
        gpioc_moder &= !(0b11 << (STATUS_LED_PIN * 2));
        gpioc_moder |= 0b01 << (STATUS_LED_PIN * 2);

        // PC10 = input for button.
        gpioc_moder &= !(0b11 << (USER_BUTTON_PIN * 2));
        write_volatile(GPIOC_MODER, gpioc_moder);

        // PC10 internal pull-up.
        let mut gpioc_pupdr = read_volatile(GPIOC_PUPDR);
        gpioc_pupdr &= !(0b11 << (USER_BUTTON_PIN * 2));
        gpioc_pupdr |= 0b01 << (USER_BUTTON_PIN * 2);
        write_volatile(GPIOC_PUPDR, gpioc_pupdr);
    }
}

fn setup_adc12_common_clock() {
    unsafe {
        // ADC12 common clock mode:
        // CKMODE = 01, synchronous clock from HCLK.
        let mut ccr = read_volatile(ADC12_CCR);
        ccr &= !(0b11 << 16);
        ccr |= 0b01 << 16;
        write_volatile(ADC12_CCR, ccr);
    }
}

fn setup_single_adc(adc_base: usize) -> u32 {
    let mut status: u32 = 0;

    unsafe {
        // Exit deep-power-down and enable ADC voltage regulator.
        let mut cr = read_volatile(adc_cr(adc_base));
        cr &= !ADC_CR_DEEPPWD;
        cr |= ADC_CR_ADVREGEN;
        write_volatile(adc_cr(adc_base), cr);

        // ADC regulator startup delay.
        asm::delay(160_000);

        // Default ADC config:
        // single conversion, right-aligned, 12-bit.
        write_volatile(adc_cfgr(adc_base), 0);

        // Calibrate ADC.
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

        // Clear ADRDY before enabling.
        write_volatile(adc_isr(adc_base), ADC_ISR_ADRDY);

        // Enable ADC.
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
    // ADC1 channels:
    // VBUS     PA0  ADC1_IN1
    // OP1_OUT  PA2  ADC1_IN3
    // TEMP     PB14 ADC1_IN5
    // POT      PB12 ADC1_IN11
    // OP3_OUT  PB1  ADC1_IN12

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

    adc1_select_channel(ADC1_BASE, POT_ADC_CHANNEL);
}

fn setup_adc2_channels() {
    // ADC2 channels:
    // OP2_OUT PA6 ADC2_IN3

    adc_set_single_ended(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    adc_set_sample_time(ADC2_BASE, OP2_OUT_ADC_CHANNEL);

    adc1_select_channel(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
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

    setup_gpio();
    let (adc1_setup_status, adc2_setup_status) = setup_adc_for_board_monitor();

    // Startup baselines. These are raw ADC comparison points only.
    let temp_startup_raw = adc_read_channel_raw(ADC1_BASE, TEMP_ADC_CHANNEL);
    let vbus_startup_raw = adc_read_channel_raw(ADC1_BASE, VBUS_ADC_CHANNEL);

    let op1_startup_raw = adc_read_channel_raw(ADC1_BASE, OP1_OUT_ADC_CHANNEL);
    let op2_startup_raw = adc_read_channel_raw(ADC2_BASE, OP2_OUT_ADC_CHANNEL);
    let op3_startup_raw = adc_read_channel_raw(ADC1_BASE, OP3_OUT_ADC_CHANNEL);

    rprintln!("================================================");
    rprintln!("B-G431B-ESC1 Rust bring-up");
    rprintln!("Version: v0.1.9-opamp-output-raw-monitor");
    rprintln!("PC6  = STATUS LED");
    rprintln!("PC10 = button input");
    rprintln!("PB12 = potentiometer / ADC1_IN11");
    rprintln!("PB14 = temperature feedback / ADC1_IN5");
    rprintln!("PA0  = VBUS feedback / ADC1_IN1");
    rprintln!("PA2  = OP1_OUT raw monitor / ADC1_IN3");
    rprintln!("PA6  = OP2_OUT raw monitor / ADC2_IN3");
    rprintln!("PB1  = OP3_OUT raw monitor / ADC1_IN12");
    rprintln!("ADC1 setup status: {}", adc1_setup_status);
    rprintln!("ADC2 setup status: {}", adc2_setup_status);
    rprintln!("temp_startup_raw: {}", temp_startup_raw);
    rprintln!("vbus_startup_raw: {}", vbus_startup_raw);
    rprintln!("op1_startup_raw: {}", op1_startup_raw);
    rprintln!("op2_startup_raw: {}", op2_startup_raw);
    rprintln!("op3_startup_raw: {}", op3_startup_raw);
    rprintln!("Note: OPAMP peripheral is not configured yet; these are raw OPx_OUT ADC pin readings.");
    rprintln!("Output format:");
    rprintln!("button=<0/1> pot_raw=<raw> pot_pct=<pct> temp_raw=<raw> vbus_raw=<raw> op1_raw=<raw> op2_raw=<raw> op3_raw=<raw> op*_delta=<raw-startup> timeout=<0/1>");
    rprintln!("================================================");

    led_low();

    loop {
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
                "button=1 pot_raw={} pot_pct={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} delay_cycles={} timeout={} mode=button_fast",
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
                "button=0 pot_raw={} pot_pct={} temp_raw={} temp_delta={} vbus_raw={} vbus_delta={} op1_raw={} op1_delta={} op2_raw={} op2_delta={} op3_raw={} op3_delta={} delay_cycles={} timeout={} mode=pot_control",
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
// Version: v0.1.9-opamp-output-raw-monitor
// Created: 2026-06-07
// Generated timestamp: 2026-06-07
// ================================================================