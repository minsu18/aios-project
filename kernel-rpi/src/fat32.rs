//! Minimal FAT32 read-only parser.
//!
//! Parses first partition from MBR, reads root directory, finds files by 8.3 name.
//! Used to load SKILL.md from SD. Uses heap for buffers when called from load_model (avoids stack overflow).

use crate::block::{BlockDevice, BlockError, BLOCK_SIZE};
use alloc::vec::Vec;

fn alloc_block() -> Vec<u8> {
    let mut v = Vec::with_capacity(BLOCK_SIZE);
    v.resize(BLOCK_SIZE, 0);
    v
}

/// 8.3 name for "SKILL.md"
pub const SKILL_MD_83: [u8; 11] = *b"SKILL   MD ";
/// 8.3 name for "MODEL.GGUF" (extension truncated to 3 chars). Used by load_model (llama feature).
#[allow(dead_code)]
pub const MODEL_GGUF_83: [u8; 11] = *b"MODEL   GGU";

/// Infer block count from MBR partition table (no read).
pub fn block_count_from_mbr_buf(buf: &[u8; BLOCK_SIZE]) -> Option<u64> {
    if buf[510] != 0x55 || buf[511] != 0xAA {
        return None;
    }
    let typ = buf[0x1C2];
    if typ != 0x0B && typ != 0x0C {
        return None;
    }
    let start =
        u32::from_le_bytes([buf[0x1C6], buf[0x1C7], buf[0x1C8], buf[0x1C9]]) as u64;
    let num =
        u32::from_le_bytes([buf[0x1CA], buf[0x1CB], buf[0x1CC], buf[0x1CD]]) as u64;
    Some(start.saturating_add(num))
}

fn mbr_partition_lba(buf: &[u8; BLOCK_SIZE]) -> Option<u64> {
    if buf[510] != 0x55 || buf[511] != 0xAA {
        return None;
    }
    let typ = buf[0x1C2];
    if typ != 0x0B && typ != 0x0C {
        return None;
    }
    let lba = u32::from_le_bytes([buf[0x1C6], buf[0x1C7], buf[0x1C8], buf[0x1C9]]);
    Some(lba as u64)
}

fn bpb_params(buf: &[u8; BLOCK_SIZE]) -> Option<(u16, u8, u32, u32, u64)> {
    let bytes_per_sector = u16::from_le_bytes([buf[11], buf[12]]);
    if bytes_per_sector != 512 {
        return None;
    }
    let sectors_per_cluster = buf[13];
    let reserved_sectors = u16::from_le_bytes([buf[14], buf[15]]);
    let num_fats = buf[16];
    let sectors_per_fat = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
    let root_cluster = u32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]);
    let data_start_lba =
        u64::from(reserved_sectors) + u64::from(num_fats) * u64::from(sectors_per_fat);
    Some((
        reserved_sectors,
        sectors_per_cluster,
        sectors_per_fat,
        root_cluster,
        data_start_lba,
    ))
}

fn eq_83(a: &[u8; 11], b: &[u8; 11]) -> bool {
    for i in 0..11 {
        let ac = a[i];
        let bc = b[i];
        if ac == b' ' && bc == b' ' {
            continue;
        }
        if ac.to_ascii_uppercase() != bc.to_ascii_uppercase() {
            return false;
        }
    }
    true
}

trait ToAsciiUpper {
    fn to_ascii_uppercase(self) -> u8;
}
impl ToAsciiUpper for u8 {
    fn to_ascii_uppercase(self) -> u8 {
        if (b'a'..=b'z').contains(&self) {
            self - 32
        } else {
            self
        }
    }
}

/// Find file in directory sector. Returns (cluster_low|high, size) if found.
fn find_in_dir(
    buf: &[u8; BLOCK_SIZE],
    name: &[u8; 11],
) -> Option<((u16, u16), u32)> {
    for ent in buf.chunks(32) {
        if ent.len() < 32 {
            break;
        }
        let first = ent[0];
        if first == 0x00 {
            break;
        }
        if first == 0xE5 {
            continue;
        }
        if ent[11] == 0x0F {
            continue;
        }
        let mut n = [0u8; 11];
        n.copy_from_slice(&ent[0..11]);
        if eq_83(&n, name) {
            let cluster_high = u16::from_le_bytes([ent[20], ent[21]]);
            let cluster_low = u16::from_le_bytes([ent[26], ent[27]]);
            let size = u32::from_le_bytes([ent[28], ent[29], ent[30], ent[31]]);
            return Some(((cluster_low, cluster_high), size));
        }
    }
    None
}

/// Read FAT entry for cluster. Reuses caller-provided buffer to avoid stack allocation.
fn read_fat_entry(
    dev: &impl BlockDevice,
    fat_start: u64,
    cluster: u32,
    buf: &mut [u8],
) -> Result<u32, BlockError> {
    let fat_offset = (u64::from(cluster)) * 4 / BLOCK_SIZE as u64;
    let sector = fat_start + fat_offset;
    dev.read_block(sector, buf)?;
    let idx = (cluster as usize % 128) * 4;
    let entry = u32::from_le_bytes([buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]);
    Ok(entry & 0x0FFF_FFFF)
}

fn cluster_to_sector(
    data_start: u64,
    sectors_per_cluster: u8,
    cluster: u32,
) -> u64 {
    data_start + u64::from(cluster.saturating_sub(2)) * u64::from(sectors_per_cluster)
}

/// Look up file in root directory. Returns (first cluster, size) if found.
/// Uses heap buffers to avoid stack overflow when called from load_model.
/// If mbr is Some, use it instead of reading block 0 (caller already has it).
pub fn find_root_file(
    dev: &impl BlockDevice,
    name_83: &[u8; 11],
    mbr: Option<&[u8; BLOCK_SIZE]>,
) -> Result<Option<(u32, u32)>, BlockError> {
    let part_start = match mbr {
        Some(m) => mbr_partition_lba(m).ok_or(BlockError::Fault("MBR"))?,
        None => {
            let mut buf = alloc_block();
            dev.read_block(0, &mut buf)?;
            let arr: &[u8; BLOCK_SIZE] = buf.as_slice().try_into().map_err(|_| BlockError::Fault("MBR"))?;
            mbr_partition_lba(arr).ok_or(BlockError::Fault("MBR"))?
        }
    };

    let mut bpb_buf = alloc_block();
    dev.read_block(part_start, &mut bpb_buf)?;
    let bpb_arr: &[u8; BLOCK_SIZE] = bpb_buf.as_slice().try_into().map_err(|_| BlockError::Fault("BPB"))?;
    let (reserved, sec_per_clu, _sec_per_fat, root_cluster, data_start_lba) =
        bpb_params(bpb_arr).ok_or(BlockError::Fault("BPB"))?;

    let fat_start = part_start + u64::from(reserved);
    let data_start = part_start + data_start_lba;

    let mut cluster = root_cluster;
    let mut seen = 0u32;
    let mut dir_buf = alloc_block();
    while cluster < 0x0FFF_FFF8 && seen < 64 {
        let sector = cluster_to_sector(data_start, sec_per_clu, cluster);
        dev.read_block(sector, &mut dir_buf)?;
        let dir_arr: &[u8; BLOCK_SIZE] = dir_buf.as_slice().try_into().map_err(|_| BlockError::Fault("dir"))?;
        if let Some(((lo, hi), size)) = find_in_dir(dir_arr, name_83) {
            let first_cluster = u32::from(lo) | (u32::from(hi) << 16);
            return Ok(Some((first_cluster, size)));
        }
        cluster = read_fat_entry(dev, fat_start, cluster, &mut dir_buf)?;
        seen += 1;
    }
    Ok(None)
}

/// Read first block of file. Call find_root_file first to get cluster.
#[allow(dead_code)]
pub fn read_file_first_block(
    dev: &impl BlockDevice,
    first_cluster: u32,
    buf: &mut [u8; BLOCK_SIZE],
) -> Result<(), BlockError> {
    let mut mbr = [0u8; BLOCK_SIZE];
    dev.read_block(0, &mut mbr)?;
    let part_start = mbr_partition_lba(&mbr).ok_or(BlockError::Fault("MBR"))?;

    let mut bpb_buf = [0u8; BLOCK_SIZE];
    dev.read_block(part_start, &mut bpb_buf)?;
    let (_reserved, sec_per_clu, _sec_per_fat, _, data_start_lba) =
        bpb_params(&bpb_buf).ok_or(BlockError::Fault("BPB"))?;

    let data_start = part_start + data_start_lba;
    let sector = cluster_to_sector(data_start, sec_per_clu, first_cluster);
    dev.read_block(sector, buf)
}

/// Read full file content into buf (up to buf.len()). Returns bytes read.
/// Uses heap for sector buffer to avoid stack overflow in load_model.
pub fn read_file_content(
    dev: &impl BlockDevice,
    first_cluster: u32,
    file_size: u32,
    buf: &mut [u8],
) -> Result<usize, BlockError> {
    let mut mbr = alloc_block();
    dev.read_block(0, &mut mbr)?;
    let mbr_arr: &[u8; BLOCK_SIZE] = mbr.as_slice().try_into().map_err(|_| BlockError::Fault("MBR"))?;
    let part_start = mbr_partition_lba(mbr_arr).ok_or(BlockError::Fault("MBR"))?;

    let mut bpb_buf = alloc_block();
    dev.read_block(part_start, &mut bpb_buf)?;
    let bpb_arr: &[u8; BLOCK_SIZE] = bpb_buf.as_slice().try_into().map_err(|_| BlockError::Fault("BPB"))?;
    let (reserved, sec_per_clu, _sec_per_fat, _, data_start_lba) =
        bpb_params(bpb_arr).ok_or(BlockError::Fault("BPB"))?;

    let fat_start = part_start + u64::from(reserved);
    let data_start = part_start + data_start_lba;
    let max_read = (file_size as usize).min(buf.len());

    let mut cluster = first_cluster;
    let mut offset = 0usize;
    let mut block = alloc_block();

    while offset < max_read && cluster >= 2 && cluster < 0x0FFF_FFF8 {
        let first_sector = cluster_to_sector(data_start, sec_per_clu, cluster);
        for s in 0..u32::from(sec_per_clu) {
            if offset >= max_read {
                break;
            }
            let sector = first_sector + u64::from(s);
            dev.read_block(sector, &mut block)?;

            let to_copy = (max_read - offset).min(BLOCK_SIZE);
            buf[offset..][..to_copy].copy_from_slice(&block[..to_copy]);
            offset += to_copy;
        }
        if offset >= max_read {
            break;
        }
        cluster = read_fat_entry(dev, fat_start, cluster, &mut block)?;
    }
    Ok(offset)
}
