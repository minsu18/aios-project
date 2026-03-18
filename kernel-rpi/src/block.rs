//! Block device abstraction for SD/MMC.
//!
//! BCM2711 EMMC2 at 0xFE34_0000. Implements SD Physical Layer spec for init and
//! single-block read. Test on real RPi 4; QEMU raspi4b does not emulate SD.

#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};

pub const BLOCK_SIZE: usize = 512;

/// Block device: read/write 512-byte sectors.
pub trait BlockDevice {
    fn read_block(&self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    fn block_count(&self) -> Option<u64>;
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

// CMDTM bits
const CMDTM_CMD_INDEX: u32 = 24;
const CMDTM_CMD_ISDATA: u32 = 1 << 21;
const CMDTM_CMD_RSPNS_48: u32 = 0 << 16;
const CMDTM_CMD_RSPNS_136: u32 = 1 << 16;
const CMDTM_CMD_RSPNS_48B: u32 = 2 << 16;
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

const TIMEOUT_LOOP: usize = 1_000_000;

/// BCM2711 EMMC2 SD host controller.
pub struct SdCard {
    base: *mut u32,
    rca: u16,           // relative card address
    is_sdhc: bool,      // block-addressed (SDHC) vs byte-addressed (SDSC)
    initialized: bool,
    block_count_cache: Option<u64>,
}

impl SdCard {
    pub const EMMC_BASE: u64 = 0xFE34_0000;

    pub fn new() -> Self {
        Self {
            base: Self::EMMC_BASE as *mut u32,
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

        let data_reg = self.reg(EMMC_DATA);
        for i in 0..(BLOCK_SIZE / 4) {
            let word = unsafe { read_volatile(data_reg.add(i)) };
            buf[i * 4..][..4].copy_from_slice(&word.to_le_bytes());
        }
        Ok(())
    }
}

impl BlockDevice for SdCard {
    fn read_block(&self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        // SdCard is not Sync; we use interior mutability pattern by making
        // read_block take &self and requiring init() before first use.
        // For simplicity we need &mut for init; after init, reads are safe.
        self.read_block_inner(offset, buf)
    }

    fn block_count(&self) -> Option<u64> {
        self.block_count_cache
    }
}
