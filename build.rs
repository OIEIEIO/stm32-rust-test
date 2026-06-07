// ================================================================
// File: build.rs
// Path: ~/stm32-rust-test/b-g431b-esc1-rust/build.rs
// Version: v0.1.0-pc6-status-blink
// Purpose: Tell rustc/linker where to find memory.x for STM32G431CB
// Target: STM32G431CB, Cortex-M4F, thumbv7em-none-eabihf
// ================================================================

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory.x");

    std::fs::copy("memory.x", out_dir.join("memory.x"))
        .expect("failed to copy memory.x to OUT_DIR");
}

// ================================================================
// Footer
// File: build.rs
// Version: v0.1.0-pc6-status-blink
// Created: 2026-06-07
// Generated timestamp: 2026-06-07
// ================================================================