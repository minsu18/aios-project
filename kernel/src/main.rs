//! AIOS Kernel — Minimal x86-64 boot entry
//!
//! Phase 2: Bare-metal entry, serial output, halt.

#![no_std]
#![no_main]

use bootloader_api::entry_point;
use bootloader_api::BootInfo;
use core::arch::asm;
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    init_serial();
    serial_write(b"AIOS kernel booted.\r\n");

    // Pass control to kernel init (Phase 2 minimal: just halt)
    loop {
        unsafe { asm!("hlt") }
    }
}

/// Initialize COM1 serial port (0x3F8)
fn init_serial() {
    unsafe {
        // Disable interrupts
        outb(0x3F8 + 1, 0x00);
        // Enable DLAB
        outb(0x3F8 + 3, 0x80);
        // Divisor low (115200 / 3 = 38400 baud)
        outb(0x3F8 + 0, 0x03);
        outb(0x3F8 + 1, 0x00);
        // 8N1, clear DLAB
        outb(0x3F8 + 3, 0x03);
        outb(0x3F8 + 2, 0xC7); // FIFO
        outb(0x3F8 + 4, 0x0B); // IRQs enabled
    }
}

fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nostack, preserves_flags));
    }
}

fn serial_write(s: &[u8]) {
    for &b in s {
        while (inb(0x3F8 + 5) & 0x20) == 0 {}
        outb(0x3F8, b);
    }
}

fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") value, options(nostack, preserves_flags));
    }
    value
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_write(b"PANIC: ");
    if let Some(s) = info.message().and_then(|m| m.as_str()) {
        serial_write(s.as_bytes());
    }
    serial_write(b"\r\n");
    loop {
        unsafe { asm!("hlt") }
    }
}
