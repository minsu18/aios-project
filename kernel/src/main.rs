//! AIOS Kernel — Minimal x86-64 boot entry
//!
//! Phase 2: Bare-metal entry, serial output, boot sequence, halt.

#![no_std]
#![no_main]

use bootloader_api::entry_point;
use bootloader_api::BootInfo;
use bootloader_api::info::MemoryRegionKind;
use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init_serial();

    serial_write(b"\r\n");
    serial_write(b"  ___    ___   ___  \r\n");
    serial_write(b" |_ _|  / _ \\ / ___|\r\n");
    serial_write(b"  | |  | | | |\\___ \\\r\n");
    serial_write(b"  | |  | |_| | ___) |\r\n");
    serial_write(b" |___|  \\___/ |____/ \r\n");
    serial_write(b" AI-Native Operating System\r\n");
    serial_write(b"\r\n");

    serial_write(b"[ 0.000] Serial init\r\n");

    let mem_mb = total_usable_mem_mb(boot_info);
    serial_write(b"[ 0.001] Memory: ");
    write_decimal(mem_mb as u64);
    serial_write(b" MB usable\r\n");

    serial_write(b"[ 0.002] HAL init (stub)\r\n");
    serial_write(b"[ 0.003] AI layer: awaiting host bridge\r\n");
    serial_write(b"\r\n>> AIOS kernel ready. (HLT)\r\n\r\n");

    loop {
        unsafe { asm!("hlt") }
    }
}

fn total_usable_mem_mb(boot_info: &BootInfo) -> u64 {
    let mut total: u64 = 0;
    for region in boot_info.memory_regions.iter() {
        if region.kind == MemoryRegionKind::Usable {
            total += region.end - region.start;
        }
    }
    total / (1024 * 1024)
}

fn write_decimal(mut n: u64) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    if n == 0 {
        serial_write(b"0");
        return;
    }
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    serial_write(&buf[i..]);
}

/// Initialize COM1 serial port (0x3F8)
fn init_serial() {
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

struct SerialWriter;
impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        serial_write(s.as_bytes());
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_write(b"PANIC: ");
    let _ = writeln!(SerialWriter, "{}", info);
    loop {
        unsafe { asm!("hlt") }
    }
}
