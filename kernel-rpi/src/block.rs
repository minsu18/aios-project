//! Block device abstraction for SD/MMC.
//!
//! Two controllers supported:
//! - BCM2711 EMMC2 at 0xFE34_0000 — real RPi 4
//! - BCM2835 SDHOST at 0xFE20_2000 — QEMU raspi4b (-drive if=sd)

#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};

pub const BLOCK_SIZE: usize = 512;

/// Block device: read/write 512-byte sectors.
pub trait BlockDevice {
    fn read_block(&self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    fn block_count(&self) -> Option<u64>;
}

/// Unified SD device: tries EMMC2 first (real RPi 4), then bcm2835-sdhost (QEMU).
pub enum SdDevice {
    Emmc2(SdCard),
    SdHost(SdHost),
}

impl SdDevice {
    /// Create and init best available SD.
    /// raspi3: only 0x3F* (avoid 0xFE* — can hang on QEMU raspi3b).
    /// raspi4: EMMC2, EMMC1, sdhost.
    pub fn new() -> Self {
        let emmc_bases: &[u64] = if cfg!(feature = "raspi3") {
            &[SdCard::EMMC1_BASE_RASPI3] // 0x3F300000 only
        } else {
            &[
                SdCard::EMMC_BASE,       // 0xFE340000 real RPi 4
                SdCard::EMMC1_BASE,       // 0xFE300000 QEMU raspi4b
            ]
        };
        for &base in emmc_bases {
            let mut emmc = SdCard::new_at(base);
            if emmc.init().is_ok() {
                return Self::Emmc2(emmc);
            }
        }
        let sdhost_bases: &[u64] = if cfg!(feature = "raspi3") {
            &[SdHost::SDHOST_BASE_RASPI3] // 0x3F202000 only
        } else {
            &[SdHost::SDHOST_BASE]
        };
        for &base in sdhost_bases {
            let mut sdhost = SdHost::new_at(base);
            if sdhost.init().is_ok() {
                return Self::SdHost(sdhost);
            }
        }
        Self::Emmc2(SdCard::new()) // all failed; caller checks is_ready()
    }

    /// True if init succeeded.
    pub fn is_ready(&self) -> bool {
        match self {
            Self::Emmc2(s) => s.is_initialized(),
            Self::SdHost(s) => s.is_initialized(),
        }
    }
}

impl BlockDevice for SdDevice {
    fn read_block(&self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        match self {
            Self::Emmc2(s) => s.read_block(offset, buf),
            Self::SdHost(s) => s.read_block(offset, buf),
        }
    }

    fn block_count(&self) -> Option<u64> {
        match self {
            Self::Emmc2(s) => s.block_count(),
            Self::SdHost(s) => s.block_count(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlockError {
    NotReady,
    Timeout,
    Fault(&'static str),
}

// --- EMMC2 register offsets (BCM2711, same layout as legacy EMMC) ---
const EMMC_ARG2: u64 = 0x00;
const EMMC_BLKSIZECNT: u64 = 0x04;
const EMMC_ARG1: u64 = 0x08;
const EMMC_CMDTM: u64 = 0x0c;
const EMMC_RESP0: u64 = 0x10;
const EMMC_RESP1: u64 = 0x14;
const EMMC_RESP2: u64 = 0x18;
const EMMC_RESP3: u64 = 0x1c;
const EMMC_DATA: u64 = 0x20;
const EMMC_STATUS: u64 = 0x24;
const EMMC_CONTROL0: u64 = 0x28;
const EMMC_CONTROL1: u64 = 0x2c;
const EMMC_INTERRUPT: u64 = 0x30;

// CMDTM bits — response type at [17:16]: 00=none, 01=136b, 10=48b, 11=48b+busy (SDHCI)
const CMDTM_CMD_INDEX: u32 = 24;
const CMDTM_CMD_ISDATA: u32 = 1 << 21;
const CMDTM_CMD_RSPNS_48: u32 = 2 << 16;  // 48-bit short response
const CMDTM_CMD_RSPNS_136: u32 = 1 << 16; // 136-bit long response
const CMDTM_CMD_RSPNS_48B: u32 = 3 << 16; // 48-bit with busy
const CMDTM_CMD_CRCCHK: u32 = 1 << 19;
const CMDTM_CMD_IXCHK: u32 = 1 << 20;
const CMDTM_TM_BLKCNT_EN: u32 = 1 << 1;
const CMDTM_TM_MULTI_BLOCK: u32 = 1 << 5;
const CMDTM_TM_DAT_DIR: u32 = 1 << 4; // 1 = host read

// STATUS bits
const STATUS_CMD_INHIBIT: u32 = 1 << 0;
const STATUS_DAT_INHIBIT: u32 = 1 << 1;
const STATUS_DAT_ACTIVE: u32 = 1 << 2;
const STATUS_WRITE_TRANSFER: u32 = 1 << 8;
const STATUS_READ_TRANSFER: u32 = 1 << 9;
const STATUS_NEW_READ_DATA: u32 = 1 << 11;

// INTERRUPT bits
const IRQ_CMD_DONE: u32 = 1 << 0;
const IRQ_DATA_DONE: u32 = 1 << 1;
const IRQ_READ_RDY: u32 = 1 << 5;
const IRQ_ERR_MASK: u32 = 0xFFFF_0000;

// CONTROL0 bits
const CTL0_PWCTL_ON: u32 = 1 << 8;
const CTL0_PWCTL_SDVOLTS_3V3: u32 = 7 << 9; // 3.3V
const CTL0_HCTL_DWIDTH: u32 = 1 << 1;       // 4-bit data

// CONTROL1 bits
const CTL1_CLK_EN: u32 = 1 << 2;
const CTL1_CLK_STABLE: u32 = 1 << 1;
const CTL1_SRST_HC: u32 = 1 << 24;
const CTL1_SRST_CMD: u32 = 1 << 25;
const CTL1_SRST_DATA: u32 = 1 << 26;
const CTL1_CLK_FREQ8: u32 = 8; // divisor: 250MHz / (2*8) ≈ 15.6 MHz init
const CTL1_CLK_INIT_DIV: u32 = 312; // ~400 kHz for init (SD spec: 100-400 kHz)

// SD commands
const CMD_GO_IDLE: u32 = 0;
const CMD_SEND_IF_COND: u32 = 8;
const CMD_SEND_CSD: u32 = 9;
const CMD_SEND_CID: u32 = 2;
const CMD_SEND_RCA: u32 = 3;
const CMD_SELECT_CARD: u32 = 7;
const CMD_SET_BLOCKLEN: u32 = 16;
const CMD_READ_BLOCK: u32 = 17;
const CMD_APP_CMD: u32 = 55;
const CMD_SD_SEND_OP_COND: u32 = 41;

/// 10k: fail fast on wrong bases; real RPi responds in ~us.
const TIMEOUT_LOOP: usize = 10_000;

/// BCM2711 EMMC2 / Arasan SDHCI controller.
pub struct SdCard {
    base: *mut u32,
    rca: u16,           // relative card address
    is_sdhc: bool,      // block-addressed (SDHC) vs byte-addressed (SDSC)
    initialized: bool,
    block_count_cache: Option<u64>,
}

impl SdCard {
    /// EMMC2 (real RPi 4)
    pub const EMMC_BASE: u64 = 0xFE34_0000;
    pub const EMMC1_BASE_RASPI3: u64 = 0x3F30_0000; // raspi3b peri_base 0x3F000000
    /// EMMC1/SDHCI (QEMU raspi4b GPIO default)
    pub const EMMC1_BASE: u64 = 0xFE30_0000;

    pub fn new() -> Self {
        Self::new_at(Self::EMMC_BASE)
    }

    pub fn new_at(base: u64) -> Self {
        Self {
            base: base as *mut u32,
            rca: 0,
            is_sdhc: false,
            initialized: false,
            block_count_cache: None,
        }
    }

    #[inline]
    fn reg(&self, off: u64) -> *mut u32 {
        unsafe { self.base.add(off as usize / 4) }
    }

    fn read_reg(&self, off: u64) -> u32 {
        unsafe { read_volatile(self.reg(off)) }
    }

    fn write_reg(&self, off: u64, val: u32) {
        unsafe { write_volatile(self.reg(off), val) }
    }

    fn wait_inhibit(&self, cmd: bool, dat: bool) -> Result<(), BlockError> {
        for _ in 0..TIMEOUT_LOOP {
            let s = self.read_reg(EMMC_STATUS);
            let cmd_inh = (s & STATUS_CMD_INHIBIT) != 0;
            let dat_inh = (s & STATUS_DAT_INHIBIT) != 0 && (s & STATUS_DAT_ACTIVE) != 0;
            if (!cmd || !cmd_inh) && (!dat || !dat_inh) {
                return Ok(());
            }
        }
        Err(BlockError::Timeout)
    }

    fn clear_irqs(&self) {
        self.write_reg(EMMC_INTERRUPT, 0xFFFF_FFFF);
    }

    fn send_cmd(&self, cmd: u32, arg: u32, flags: u32) -> Result<u32, BlockError> {
        self.wait_inhibit(true, false)?;
        self.clear_irqs();
        self.write_reg(EMMC_ARG1, arg);
        self.write_reg(EMMC_ARG2, 0);
        self.write_reg(
            EMMC_CMDTM,
            (cmd << CMDTM_CMD_INDEX) | flags,
        );
        for _ in 0..TIMEOUT_LOOP {
            let irq = self.read_reg(EMMC_INTERRUPT);
            if (irq & IRQ_ERR_MASK) != 0 {
                self.write_reg(EMMC_INTERRUPT, irq);
                return Err(BlockError::Fault("CMD error"));
            }
            if (irq & IRQ_CMD_DONE) != 0 {
                self.write_reg(EMMC_INTERRUPT, irq);
                return Ok(self.read_reg(EMMC_RESP0));
            }
        }
        Err(BlockError::Timeout)
    }

    /// Send command with 136-bit R2 response (e.g. CMD9 SEND_CSD).
    fn send_cmd_r2(&self, cmd: u32, arg: u32) -> Result<[u32; 4], BlockError> {
        self.wait_inhibit(true, false)?;
        self.clear_irqs();
        self.write_reg(EMMC_ARG1, arg);
        self.write_reg(EMMC_ARG2, 0);
        self.write_reg(
            EMMC_CMDTM,
            (cmd << CMDTM_CMD_INDEX) | CMDTM_CMD_RSPNS_136,
        );
        for _ in 0..TIMEOUT_LOOP {
            let irq = self.read_reg(EMMC_INTERRUPT);
            if (irq & IRQ_ERR_MASK) != 0 {
                self.write_reg(EMMC_INTERRUPT, irq);
                return Err(BlockError::Fault("CMD error"));
            }
            if (irq & IRQ_CMD_DONE) != 0 {
                self.write_reg(EMMC_INTERRUPT, irq);
                return Ok([
                    self.read_reg(EMMC_RESP0),
                    self.read_reg(EMMC_RESP1),
                    self.read_reg(EMMC_RESP2),
                    self.read_reg(EMMC_RESP3),
                ]);
            }
        }
        Err(BlockError::Timeout)
    }

    /// Parse CSD (128 bits) into block count. RESP0..3 = CSD [127:96],[95:64],[63:32],[31:0].
    fn parse_csd_blocks(r: &[u32; 4]) -> u64 {
        let structure = (r[0] >> 30) & 3;
        if structure == 0 {
            // CSD v1: C_SIZE 73-62, C_SIZE_MULT 49-47, READ_BL_LEN 83-80
            let c_size = ((r[1] & 0x3FF) << 2) | (r[2] >> 30);
            let c_size_mult = (r[2] >> 15) & 7;
            let read_bl_len = (r[1] >> 16) & 0xF;
            let mult = 1u64 << (c_size_mult + 2);
            let blk = 1u64 << read_bl_len;
            (u64::from(c_size) + 1) * mult * blk / BLOCK_SIZE as u64
        } else {
            // CSD v2: C_SIZE 69-48, capacity = (C_SIZE+1)*512*1024 bytes
            let c_size = ((r[1] & 0x3F) << 16) | (r[2] >> 16);
            ((u64::from(c_size) + 1) * 512 * 1024) / BLOCK_SIZE as u64
        }
    }

    fn init_clock(&mut self) -> Result<(), BlockError> {
        self.write_reg(EMMC_CONTROL1, CTL1_SRST_HC);
        for _ in 0..TIMEOUT_LOOP {
            if self.read_reg(EMMC_CONTROL1) & CTL1_SRST_HC == 0 {
                break;
            }
        }
        // Start with slow clock (~400 kHz) for init
        self.write_reg(EMMC_CONTROL1, CTL1_CLK_INIT_DIV << 8);
        self.write_reg(EMMC_CONTROL1, self.read_reg(EMMC_CONTROL1) | CTL1_CLK_EN);
        for _ in 0..TIMEOUT_LOOP {
            if (self.read_reg(EMMC_CONTROL1) & CTL1_CLK_STABLE) != 0 {
                break;
            }
        }
        Ok(())
    }

    fn init_power(&mut self) -> Result<(), BlockError> {
        self.wait_inhibit(true, true)?;
        self.write_reg(
            EMMC_CONTROL0,
            CTL0_PWCTL_ON | CTL0_PWCTL_SDVOLTS_3V3,
        );
        for _ in 0..TIMEOUT_LOOP {
            let s = self.read_reg(EMMC_CONTROL0);
            if (s & CTL0_PWCTL_ON) != 0 {
                return Ok(());
            }
        }
        Err(BlockError::Timeout)
    }

    /// Full SD initialization: clock, power, CMD0/8/55+41, CMD2/3/7/16.
    pub fn init(&mut self) -> Result<(), BlockError> {
        if self.initialized {
            return Ok(());
        }

        self.init_clock()?;
        self.init_power()?;

        // CMD0: go idle
        self.send_cmd(CMD_GO_IDLE, 0, 0)?;

        // CMD8: voltage check (0x1AA), R7
        let r7 = self.send_cmd(
            CMD_SEND_IF_COND,
            0x1AA,
            CMDTM_CMD_RSPNS_48 | CMDTM_CMD_CRCCHK | CMDTM_CMD_IXCHK,
        );
        // If illegal command (old card), continue without CMD8
        if r7.is_err() {
            // Try CMD1 for MMC, but we target SD — assume SDHC path
        }

        // ACMD41: send op cond (HCS=1 for SDHC). Poll until ready.
        let mut retries = 50;
        while retries > 0 {
            self.send_cmd(CMD_APP_CMD, u32::from(self.rca) << 16, CMDTM_CMD_RSPNS_48)?;
            let r3 = self.send_cmd(
                CMD_SD_SEND_OP_COND,
                0x40FF_8000, // HCS=1, S18R=0, 3.2-3.3V
                CMDTM_CMD_RSPNS_48,
            )?;
            if r3 & 0x8000_0000 != 0 {
                // power-up busy clear
                self.is_sdhc = (r3 & 0x4000_0000) != 0; // CCS
                break;
            }
            retries -= 1;
            for _ in 0..10000 {}
        }
        if retries == 0 {
            return Err(BlockError::Fault("ACMD41 timeout"));
        }

        // CMD2: get CID
        self.send_cmd(CMD_SEND_CID, 0, CMDTM_CMD_RSPNS_136)?;

        // CMD3: get RCA
        let r6 = self.send_cmd(CMD_SEND_RCA, 0, CMDTM_CMD_RSPNS_48)?;
        self.rca = (r6 >> 16) as u16;
        if self.rca == 0 {
            return Err(BlockError::Fault("no RCA"));
        }

        // CMD7: select card
        self.send_cmd(CMD_SELECT_CARD, u32::from(self.rca) << 16, CMDTM_CMD_RSPNS_48)?;

        // CMD9: get CSD for capacity
        let csd = self.send_cmd_r2(CMD_SEND_CSD, u32::from(self.rca) << 16)?;
        self.block_count_cache = Some(Self::parse_csd_blocks(&csd));

        // CMD16: set block length (512) — needed for SDSC; SDHC has fixed 512
        self.send_cmd(CMD_SET_BLOCKLEN, BLOCK_SIZE as u32, CMDTM_CMD_RSPNS_48)?;

        // 4-bit bus and step up clock for normal operation
        self.write_reg(EMMC_CONTROL0, self.read_reg(EMMC_CONTROL0) | CTL0_HCTL_DWIDTH);
        self.write_reg(
            EMMC_CONTROL1,
            (self.read_reg(EMMC_CONTROL1) & !0xFF00) | (CTL1_CLK_FREQ8 << 8),
        );

        self.initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn read_block_inner(&self, block_index: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if !self.initialized {
            return Err(BlockError::NotReady);
        }
        if buf.len() < BLOCK_SIZE {
            return Err(BlockError::Fault("buf too small"));
        }

        let arg = if self.is_sdhc {
            block_index
        } else {
            block_index * BLOCK_SIZE as u64
        };

        self.wait_inhibit(true, true)?;
        self.clear_irqs();
        self.write_reg(EMMC_BLKSIZECNT, (BLOCK_SIZE as u32) | (1 << 16));
        self.write_reg(EMMC_ARG1, arg as u32);
        self.write_reg(EMMC_ARG2, (arg >> 32) as u32);
        self.write_reg(
            EMMC_CMDTM,
            (CMD_READ_BLOCK << CMDTM_CMD_INDEX)
                | CMDTM_CMD_ISDATA
                | CMDTM_CMD_RSPNS_48
                | CMDTM_CMD_CRCCHK
                | CMDTM_TM_BLKCNT_EN
                | CMDTM_TM_DAT_DIR,
        );

        for _ in 0..TIMEOUT_LOOP {
            let irq = self.read_reg(EMMC_INTERRUPT);
            if (irq & IRQ_ERR_MASK) != 0 {
                self.write_reg(EMMC_INTERRUPT, irq);
                return Err(BlockError::Fault("read error"));
            }
            if (irq & IRQ_DATA_DONE) != 0 {
                self.write_reg(EMMC_INTERRUPT, irq);
                break;
            }
        }

        // DATA at 0x20 is a FIFO — read repeatedly from same address (not .add(i))
        let data_reg = self.reg(EMMC_DATA);
        for i in 0..(BLOCK_SIZE / 4) {
            let word = unsafe { read_volatile(data_reg) };
            buf[i * 4..][..4].copy_from_slice(&word.to_le_bytes());
        }
        Ok(())
    }
}

impl BlockDevice for SdCard {
    fn read_block(&self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        self.read_block_inner(offset, buf)
    }

    fn block_count(&self) -> Option<u64> {
        self.block_count_cache
    }
}

// --- BCM2835 SDHOST (QEMU raspi4b at 0xFE20_2000) ---
const SDHOST_CMD: u64 = 0x00;
const SDHOST_ARG: u64 = 0x04;
const SDHOST_RSP0: u64 = 0x10;
const SDHOST_RSP1: u64 = 0x14;
const SDHOST_RSP2: u64 = 0x18;
const SDHOST_RSP3: u64 = 0x1c;
const SDHOST_HSTS: u64 = 0x20;
const SDHOST_VDD: u64 = 0x30;
const SDHOST_EDM: u64 = 0x34;
const SDHOST_HCFG: u64 = 0x38;
const SDHOST_HBCT: u64 = 0x3c;
const SDHOST_DATA: u64 = 0x40;
const SDHOST_HBLC: u64 = 0x50;

const SDHOST_CMD_NEW: u32 = 0x8000;
const SDHOST_CMD_FAIL: u32 = 0x4000;
const SDHOST_CMD_NO_RESP: u32 = 0x400;
const SDHOST_CMD_READ: u32 = 0x40;
const SDHOST_CMD_MASK: u32 = 0x3f;

const SDHOST_HSTS_ERR: u32 = 0xF8; // timeouts, CRC, FIFO
const SDHOST_HSTS_BUSY: u32 = 0x400;

/// BCM2835 SDHOST controller (QEMU raspi4b maps SD here).
pub struct SdHost {
    base: *mut u32,
    rca: u16,
    is_sdhc: bool,
    initialized: bool,
    block_count_cache: Option<u64>,
}

impl SdHost {
    /// raspi4b (BCM2838): peri_base 0xFE000000
    pub const SDHOST_BASE: u64 = 0xFE20_2000;
    /// raspi3b (BCM2837): peri_base 0x3F000000
    pub const SDHOST_BASE_RASPI3: u64 = 0x3F20_2000;

    pub fn new() -> Self {
        Self::new_at(Self::SDHOST_BASE)
    }

    pub fn new_at(base: u64) -> Self {
        Self {
            base: base as *mut u32,
            rca: 0,
            is_sdhc: false,
            initialized: false,
            block_count_cache: None,
        }
    }

    #[inline]
    fn reg(&self, off: u64) -> *mut u32 {
        unsafe { self.base.add(off as usize / 4) }
    }

    fn read_reg(&self, off: u64) -> u32 {
        unsafe { read_volatile(self.reg(off)) }
    }

    fn write_reg(&self, off: u64, val: u32) {
        unsafe { write_volatile(self.reg(off), val) }
    }

    fn clear_status(&self, mask: u32) {
        self.write_reg(SDHOST_HSTS, mask);
    }

    fn send_cmd(&self, cmd: u32, arg: u32, flags: u32) -> Result<u32, BlockError> {
        self.clear_status(SDHOST_HSTS_ERR);
        self.write_reg(SDHOST_ARG, arg);
        self.write_reg(SDHOST_CMD, (cmd & SDHOST_CMD_MASK) | flags | SDHOST_CMD_NEW);
        for _ in 0..TIMEOUT_LOOP {
            let c = self.read_reg(SDHOST_CMD);
            if (c & SDHOST_CMD_NEW) == 0 {
                let hsts = self.read_reg(SDHOST_HSTS);
                if (hsts & SDHOST_HSTS_ERR) != 0 {
                    return Err(BlockError::Fault("sdhost CMD err"));
                }
                return Ok(self.read_reg(SDHOST_RSP0));
            }
        }
        Err(BlockError::Timeout)
    }

    fn send_cmd_r2(&self, cmd: u32, arg: u32) -> Result<[u32; 4], BlockError> {
        self.clear_status(SDHOST_HSTS_ERR);
        self.write_reg(SDHOST_ARG, arg);
        self.write_reg(
            SDHOST_CMD,
            (cmd & SDHOST_CMD_MASK) | 0x200 /* long resp */ | SDHOST_CMD_NEW,
        );
        for _ in 0..TIMEOUT_LOOP {
            let c = self.read_reg(SDHOST_CMD);
            if (c & SDHOST_CMD_NEW) == 0 {
                let hsts = self.read_reg(SDHOST_HSTS);
                if (hsts & SDHOST_HSTS_ERR) != 0 {
                    return Err(BlockError::Fault("sdhost R2 err"));
                }
                return Ok([
                    self.read_reg(SDHOST_RSP0),
                    self.read_reg(SDHOST_RSP1),
                    self.read_reg(SDHOST_RSP2),
                    self.read_reg(SDHOST_RSP3),
                ]);
            }
        }
        Err(BlockError::Timeout)
    }

    fn parse_csd_blocks(r: &[u32; 4]) -> u64 {
        let structure = (r[0] >> 30) & 3;
        if structure == 0 {
            let c_size = ((r[1] & 0x3FF) << 2) | (r[2] >> 30);
            let c_size_mult = (r[2] >> 15) & 7;
            let read_bl_len = (r[1] >> 16) & 0xF;
            let mult = 1u64 << (c_size_mult + 2);
            let blk = 1u64 << read_bl_len;
            (u64::from(c_size) + 1) * mult * blk / BLOCK_SIZE as u64
        } else {
            let c_size = ((r[1] & 0x3F) << 16) | (r[2] >> 16);
            ((u64::from(c_size) + 1) * 512 * 1024) / BLOCK_SIZE as u64
        }
    }

    pub fn init(&mut self) -> Result<(), BlockError> {
        if self.initialized {
            return Ok(());
        }
        self.clear_status(0x7FF);
        self.write_reg(SDHOST_VDD, 1);
        for _ in 0..1000 {}
        self.send_cmd(CMD_GO_IDLE, 0, SDHOST_CMD_NO_RESP)?;
        let _ = self.send_cmd(CMD_SEND_IF_COND, 0x1AA, 0); /* R7 optional */
        let mut retries = 50;
        while retries > 0 {
            self.send_cmd(CMD_APP_CMD, u32::from(self.rca) << 16, 0)?;
            let r3 = self.send_cmd(CMD_SD_SEND_OP_COND, 0x40FF_8000, 0)?;
            if r3 & 0x8000_0000 != 0 {
                self.is_sdhc = (r3 & 0x4000_0000) != 0;
                break;
            }
            retries -= 1;
            for _ in 0..10000 {}
        }
        if retries == 0 {
            return Err(BlockError::Fault("ACMD41 timeout"));
        }
        self.send_cmd(CMD_SEND_CID, 0, 0x200)?;
        let r6 = self.send_cmd(CMD_SEND_RCA, 0, 0)?;
        self.rca = (r6 >> 16) as u16;
        if self.rca == 0 {
            return Err(BlockError::Fault("no RCA"));
        }
        self.send_cmd(CMD_SELECT_CARD, u32::from(self.rca) << 16, 0)?;
        let csd = self.send_cmd_r2(CMD_SEND_CSD, u32::from(self.rca) << 16)?;
        self.block_count_cache = Some(Self::parse_csd_blocks(&csd));
        self.send_cmd(CMD_SET_BLOCKLEN, BLOCK_SIZE as u32, 0)?;
        self.initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn read_block_inner(&self, block_index: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if !self.initialized {
            return Err(BlockError::NotReady);
        }
        if buf.len() < BLOCK_SIZE {
            return Err(BlockError::Fault("buf too small"));
        }
        let arg = if self.is_sdhc {
            block_index as u32
        } else {
            (block_index * BLOCK_SIZE as u64) as u32
        };
        self.clear_status(SDHOST_HSTS_ERR);
        self.write_reg(SDHOST_HBCT, BLOCK_SIZE as u32);
        self.write_reg(SDHOST_HBLC, 1);
        self.write_reg(SDHOST_ARG, arg);
        self.write_reg(
            SDHOST_CMD,
            (CMD_READ_BLOCK & SDHOST_CMD_MASK) | SDHOST_CMD_READ | SDHOST_CMD_NEW,
        );
        for _ in 0..TIMEOUT_LOOP {
            let c = self.read_reg(SDHOST_CMD);
            if (c & SDHOST_CMD_NEW) == 0 {
                if (c & SDHOST_CMD_FAIL) != 0 {
                    return Err(BlockError::Fault("sdhost read err"));
                }
                break;
            }
        }
        let data_reg = self.reg(SDHOST_DATA);
        for i in 0..(BLOCK_SIZE / 4) {
            let word = unsafe { read_volatile(data_reg) };
            buf[i * 4..][..4].copy_from_slice(&word.to_le_bytes());
        }
        Ok(())
    }
}

impl BlockDevice for SdHost {
    fn read_block(&self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        self.read_block_inner(offset, buf)
    }

    fn block_count(&self) -> Option<u64> {
        self.block_count_cache
    }
}