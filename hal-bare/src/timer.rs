//! Timer abstraction — ARM Generic Timer

/// Read ARM Generic Timer: (CNTVCT_EL0, CNTFRQ_EL0)
/// Returns (ticks, frequency_hz)
#[inline(always)]
pub fn read() -> (u64, u64) {
    let ticks: u64;
    let freq: u64;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        ticks = 0;
        freq = 0;
    }
    (ticks, freq)
}
