//! Power management — reset, watchdog (BCM2711)

/// BCM2711 PM base (0x7E100000 in VC space -> 0xFE100000 on ARM)
const PM_BASE: u64 = 0xFE10_0000;
const PM_RSTC: u64 = 0x1C;
const PM_WDOG: u64 = 0x24;
const PM_PASSWORD: u32 = 0x5A00_0000;
const PM_RSTC_WRCFG_FULL_RESET: u32 = 0x20;

/// Trigger full system reset. Does not return.
pub fn reboot() -> ! {
    let rstc = (PM_BASE + PM_RSTC) as *mut u32;
    let wdog = (PM_BASE + PM_WDOG) as *mut u32;
    unsafe {
        wdog.write_volatile(PM_PASSWORD | 1); // 1 tick
        rstc.write_volatile(PM_PASSWORD | PM_RSTC_WRCFG_FULL_RESET); // WRCFG at bits [5:4], no shift
    }
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}
