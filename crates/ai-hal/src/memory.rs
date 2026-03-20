// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! # memory
//!
//! Linux 메모리 직접 제어 HAL 구현.
//!
//! ## 구현 기술
//! - **상태 조회**: `/proc/meminfo` 파싱 (MemTotal, MemAvailable, Buffers, Cached)
//! - **메모리 할당**: `mmap(2)` syscall — MAP_ANONYMOUS | MAP_PRIVATE (또는 MAP_SHARED)
//! - **메모리 해제**: `munmap(2)` syscall
//! - **페이지 크기**: `sysconf(_SC_PAGESIZE)` syscall
//!
//! ## 안전성 참고
//! `mmap`/`munmap`은 unsafe 블록에서 호출되며,
//! 각 unsafe 블록에 // SAFETY: 주석으로 안전성 근거를 명시함.
//!
//! ## 참조
//! - mmap(2): https://man7.org/linux/man-pages/man2/mmap.2.html
//! - /proc/meminfo: https://www.kernel.org/doc/Documentation/filesystems/proc.txt

use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::{HalError, HalResponse, MemoryHandle, MemoryState, ResourceState};

// ─────────────────────────────────────────────
//  mmap 할당 레코드
// ─────────────────────────────────────────────

/// mmap으로 할당된 메모리 영역의 내부 레코드.
///
/// MemoryHandle → MmapRecord 매핑으로 munmap 시 정확한 해제 보장.
struct MmapRecord {
    /// mmap이 반환한 포인터 (munmap에 전달)
    ptr: *mut libc::c_void,
    /// 할당 크기 (페이지 정렬됨)
    aligned_size: usize,
}

// SAFETY: MmapRecord 내부 포인터는 LinuxMemoryHal의 Mutex로 보호됨.
// 동시 접근 시 락을 통해 독점적 접근 보장.
unsafe impl Send for MmapRecord {}
unsafe impl Sync for MmapRecord {}

// ─────────────────────────────────────────────
//  LinuxMemoryHal 구조체
// ─────────────────────────────────────────────

/// Linux 메모리 직접 제어 HAL.
///
/// ## 스레드 안전성
/// - `allocations`: Mutex로 보호 — 멀티스레드 Skill 런타임 지원
/// - `next_id`: AtomicU64 — 락 없이 ID 생성
pub struct LinuxMemoryHal {
    /// mmap 할당 추적 테이블: MemoryHandle.raw_id() → MmapRecord
    allocations: Mutex<HashMap<u64, MmapRecord>>,
    /// 다음 발급할 핸들 ID (단조 증가)
    next_id: AtomicU64,
}

impl LinuxMemoryHal {
    /// 새 LinuxMemoryHal 인스턴스 생성.
    pub fn new() -> Self {
        Self {
            allocations: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    // ─────────────────────────────────────────
    //  공개 인터페이스
    // ─────────────────────────────────────────

    /// `/proc/meminfo` 파싱으로 현재 메모리 상태 조회.
    ///
    /// ## 알고리즘
    /// 1. `/proc/meminfo` 파일 읽기
    /// 2. `key: value kB` 형식 파싱
    /// 3. MemTotal, MemAvailable, Buffers, Cached 추출
    /// 4. used = total - available
    ///
    /// ## 참조
    /// https://www.kernel.org/doc/Documentation/filesystems/proc.txt (Section 2.1)
    pub fn query_state_inner(&self) -> Result<HalResponse, HalError> {
        let state = Self::read_meminfo()?;
        Ok(HalResponse::ResourceState(ResourceState::Memory(state)))
    }

    /// 메모리 영역을 mmap으로 할당.
    ///
    /// ## 알고리즘
    /// 1. 요청 크기를 페이지 경계로 올림 (page_size의 배수)
    /// 2. alignment가 페이지 크기 이상이면 MAP_FIXED 없이 커널에 위임
    /// 3. mmap(2) 호출: MAP_ANONYMOUS | MAP_PRIVATE (또는 MAP_SHARED)
    /// 4. 핸들 발급 및 allocations 테이블에 등록
    ///
    /// # 인자
    /// - `size_bytes`: 요청 바이트 수
    /// - `alignment`: 정렬 요구 (2의 거듭제곱, 최소 PAGE_SIZE)
    /// - `shared`: true → MAP_SHARED, false → MAP_PRIVATE
    pub fn allocate(
        &self,
        size_bytes: usize,
        alignment: usize,
        shared: bool,
    ) -> Result<MemoryHandle, HalError> {
        // 파라미터 검증
        if size_bytes == 0 {
            return Err(HalError::InvalidParameter {
                param_name: "size_bytes".to_string(),
                message: "0바이트 할당은 불가합니다".to_string(),
            });
        }
        if alignment != 0 && !alignment.is_power_of_two() {
            return Err(HalError::InvalidParameter {
                param_name: "alignment".to_string(),
                message: "alignment은 2의 거듭제곱이어야 합니다".to_string(),
            });
        }

        let page_size = Self::page_size();
        // 요청 크기를 페이지 크기의 배수로 올림 (ceiling division)
        let aligned_size = size_bytes.div_ceil(page_size) * page_size;

        // mmap 플래그 설정
        let map_flags = if shared {
            libc::MAP_ANONYMOUS | libc::MAP_SHARED
        } else {
            libc::MAP_ANONYMOUS | libc::MAP_PRIVATE
        };

        // SAFETY:
        // 1. addr = NULL → 커널이 적절한 주소 선택
        // 2. MAP_ANONYMOUS: 파일 없이 순수 익명 메모리
        // 3. PROT_READ | PROT_WRITE: 읽기/쓰기 권한
        // 4. fd = -1, offset = 0: MAP_ANONYMOUS 요구사항
        // 5. 반환값 MAP_FAILED 검사로 에러 처리
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),  // 커널이 주소 선택
                aligned_size,
                libc::PROT_READ | libc::PROT_WRITE,
                map_flags,
                -1,  // MAP_ANONYMOUS: fd 불필요
                0,   // MAP_ANONYMOUS: offset 불필요
            )
        };

        // MAP_FAILED = (void*)-1 검사
        if ptr == libc::MAP_FAILED {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return Err(HalError::SyscallFailed {
                syscall: "mmap",
                errno,
                message: format!(
                    "mmap 실패: {}바이트 할당 불가",
                    aligned_size
                ),
            });
        }

        // 핸들 발급 (단조 증가 ID)
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let handle = MemoryHandle::new(id);

        // 할당 테이블에 등록
        self.allocations
            .lock()
            .map_err(|_| HalError::InternalError("allocations 락 획득 실패".to_string()))?
            .insert(id, MmapRecord { ptr, aligned_size });

        Ok(handle)
    }

    /// mmap으로 할당된 메모리를 munmap으로 해제.
    ///
    /// ## 알고리즘
    /// 1. 핸들로 allocations 테이블에서 MmapRecord 검색
    /// 2. munmap(2) 호출
    /// 3. 테이블에서 레코드 제거
    pub fn free(&self, handle: MemoryHandle) -> Result<(), HalError> {
        let mut table = self
            .allocations
            .lock()
            .map_err(|_| HalError::InternalError("allocations 락 획득 실패".to_string()))?;

        let record = table.remove(&handle.raw_id()).ok_or_else(|| {
            HalError::InvalidParameter {
                param_name: "handle".to_string(),
                message: format!("핸들 {} 를 찾을 수 없습니다", handle.raw_id()),
            }
        })?;

        // SAFETY:
        // 1. ptr은 mmap이 반환한 유효한 포인터
        // 2. aligned_size는 mmap 호출 시 사용한 크기와 동일
        // 3. table.remove() 후 ptr 소유권을 확보했으므로 중복 해제 불가
        let ret = unsafe { libc::munmap(record.ptr, record.aligned_size) };

        if ret != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return Err(HalError::SyscallFailed {
                syscall: "munmap",
                errno,
                message: format!("munmap 실패: 핸들 {}", handle.raw_id()),
            });
        }

        Ok(())
    }

    /// 현재 할당된 메모리 핸들 수 반환 (디버깅/모니터링용).
    pub fn allocation_count(&self) -> usize {
        self.allocations.lock().map(|t| t.len()).unwrap_or(0)
    }

    // ─────────────────────────────────────────
    //  내부 헬퍼 함수
    // ─────────────────────────────────────────

    /// `/proc/meminfo`를 파싱하여 MemoryState 반환.
    ///
    /// ## 파싱 대상 필드
    /// ```text
    /// MemTotal:       16384000 kB   ← 총 물리 메모리
    /// MemFree:         4096000 kB
    /// MemAvailable:    9000000 kB   ← 실제 사용 가능 (캐시 포함)
    /// Buffers:          500000 kB   ← 버퍼 캐시
    /// Cached:          2000000 kB   ← 페이지 캐시
    /// ```
    pub fn read_meminfo() -> Result<MemoryState, HalError> {
        let content = fs::read_to_string("/proc/meminfo").map_err(|e| HalError::SyscallFailed {
            syscall: "read(/proc/meminfo)",
            errno: e.raw_os_error().unwrap_or(0),
            message: format!("/proc/meminfo 읽기 실패: {}", e),
        })?;

        // key → kB 값 파싱 테이블 구성
        let mut fields: HashMap<&str, u64> = HashMap::new();
        for line in content.lines() {
            // 형식: "MemTotal:       16384000 kB"
            let mut parts = line.splitn(2, ':');
            let key = parts.next().unwrap_or("").trim();
            if let Some(val_str) = parts.next() {
                // "16384000 kB" → 16384000
                let val: u64 = val_str
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
                fields.insert(key, val);
            }
        }

        // kB → bytes 변환 헬퍼
        let kb_to_bytes = |key: &str| fields.get(key).copied().unwrap_or(0) * 1024;

        let total_bytes     = kb_to_bytes("MemTotal");
        let available_bytes = kb_to_bytes("MemAvailable");
        let buffers_bytes   = kb_to_bytes("Buffers");
        let cached_bytes    = kb_to_bytes("Cached");

        // 사용 중인 메모리 = 총 메모리 - 사용 가능한 메모리
        let used_bytes = total_bytes.saturating_sub(available_bytes);

        // 페이지 크기: sysconf(_SC_PAGESIZE) 또는 기본값 4096
        let page_size = Self::page_size();

        Ok(MemoryState {
            total_bytes,
            used_bytes,
            available_bytes,
            buffers_bytes,
            cached_bytes,
            page_size,
        })
    }

    /// 시스템 페이지 크기를 반환.
    ///
    /// `sysconf(_SC_PAGESIZE)` syscall 사용.
    /// 실패 시 기본값 4096 반환.
    fn page_size() -> usize {
        // SAFETY: sysconf는 항상 안전하게 호출 가능한 POSIX 함수
        let ps = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if ps > 0 { ps as usize } else { 4096 }
    }
}

impl Default for LinuxMemoryHal {
    fn default() -> Self { Self::new() }
}

/// 소멸 시 미해제 mmap 영역 경고 출력.
///
/// 메모리 누수 디버깅을 위해 Drop에서 감지.
impl Drop for LinuxMemoryHal {
    fn drop(&mut self) {
        if let Ok(table) = self.allocations.lock() {
            if !table.is_empty() {
                eprintln!(
                    "[ai-hal] 경고: LinuxMemoryHal 소멸 시 {}개 mmap 영역이 해제되지 않았습니다.",
                    table.len()
                );
                // 남은 영역 일괄 해제
                for (id, record) in table.iter() {
                    // SAFETY: Drop에서 호출, 이 시점에 다른 참조자 없음
                    let ret = unsafe { libc::munmap(record.ptr, record.aligned_size) };
                    if ret != 0 {
                        eprintln!("  [ai-hal] munmap 실패: 핸들 {}", id);
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────
//  단위 테스트
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── /proc/meminfo 파싱 테스트 ──────────────

    #[test]
    fn test_read_meminfo_returns_valid_state() {
        // 실제 /proc/meminfo 파싱 결과 검증 (Linux 환경)
        let result = LinuxMemoryHal::read_meminfo();
        assert!(result.is_ok(), "/proc/meminfo 파싱 실패: {:?}", result.err());

        let mem = result.unwrap();
        // 총 메모리는 0보다 커야 함
        assert!(mem.total_bytes > 0, "total_bytes가 0입니다");
        // available은 total 이하여야 함
        assert!(
            mem.available_bytes <= mem.total_bytes,
            "available > total: {} > {}",
            mem.available_bytes,
            mem.total_bytes
        );
        // 페이지 크기는 512 이상 (최소 512바이트)
        assert!(mem.page_size >= 512);
        // 사용률 검증 (0.0 ~ 1.0)
        let ratio = mem.usage_ratio();
        assert!(ratio >= 0.0 && ratio <= 1.0, "사용률 범위 초과: {}", ratio);
    }

    // ── mmap 할당/해제 테스트 ──────────────────

    #[test]
    fn test_allocate_and_free_basic() {
        // 기본 메모리 할당 및 해제 검증
        let hal = LinuxMemoryHal::new();

        let handle = hal.allocate(4096, 4096, false);
        assert!(handle.is_ok(), "mmap 실패: {:?}", handle.err());

        let h = handle.unwrap();
        assert_eq!(hal.allocation_count(), 1);

        let free_result = hal.free(h);
        assert!(free_result.is_ok(), "munmap 실패: {:?}", free_result.err());
        assert_eq!(hal.allocation_count(), 0);
    }

    #[test]
    fn test_allocate_1mb_shared() {
        // 1MB 공유 메모리 할당 검증
        let hal = LinuxMemoryHal::new();
        let handle = hal.allocate(1024 * 1024, 4096, true);
        assert!(handle.is_ok());
        let _ = hal.free(handle.unwrap());
    }

    #[test]
    fn test_allocate_multiple_handles_unique() {
        // 복수 할당 시 핸들 ID가 모두 다른지 확인
        let hal = LinuxMemoryHal::new();
        let h1 = hal.allocate(4096, 4096, false).unwrap();
        let h2 = hal.allocate(8192, 4096, false).unwrap();
        let h3 = hal.allocate(4096, 4096, false).unwrap();

        assert_ne!(h1.raw_id(), h2.raw_id());
        assert_ne!(h2.raw_id(), h3.raw_id());
        assert_eq!(hal.allocation_count(), 3);

        let _ = hal.free(h1);
        let _ = hal.free(h2);
        let _ = hal.free(h3);
        assert_eq!(hal.allocation_count(), 0);
    }

    #[test]
    fn test_allocate_zero_size_returns_error() {
        // 0바이트 할당 요청 → InvalidParameter 에러
        let hal = LinuxMemoryHal::new();
        let result = hal.allocate(0, 4096, false);
        assert!(matches!(result, Err(HalError::InvalidParameter { .. })));
    }

    #[test]
    fn test_allocate_non_power_of_two_alignment_error() {
        // alignment가 2의 거듭제곱이 아니면 InvalidParameter
        let hal = LinuxMemoryHal::new();
        let result = hal.allocate(4096, 3000, false); // 3000: 2의 거듭제곱 아님
        assert!(matches!(result, Err(HalError::InvalidParameter { param_name, .. })
            if param_name == "alignment"));
    }

    #[test]
    fn test_free_invalid_handle_returns_error() {
        // 존재하지 않는 핸들 해제 → InvalidParameter 에러
        let hal = LinuxMemoryHal::new();
        let fake_handle = MemoryHandle::new(9999);
        let result = hal.free(fake_handle);
        assert!(matches!(result, Err(HalError::InvalidParameter { .. })));
    }

    #[test]
    fn test_page_size_is_reasonable() {
        // 페이지 크기가 합리적인 범위인지 확인 (512 ~ 65536)
        let ps = LinuxMemoryHal::page_size();
        assert!(ps >= 512 && ps <= 65536, "비정상 페이지 크기: {}", ps);
    }

    #[test]
    fn test_query_state_inner_returns_memory_state() {
        // query_state_inner()가 유효한 Memory 상태 반환하는지 확인
        let hal = LinuxMemoryHal::new();
        let result = hal.query_state_inner();
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            HalResponse::ResourceState(crate::ResourceState::Memory(_))
        ));
    }

    // ── 대용량 할당 경계 테스트 ────────────────

    #[test]
    fn test_allocate_non_page_aligned_size_rounds_up() {
        // 페이지 미정렬 크기가 자동으로 올림되는지 확인
        // 예: 100바이트 요청 → 실제 4096바이트 할당
        let hal = LinuxMemoryHal::new();
        let handle = hal.allocate(100, 4096, false);
        assert!(handle.is_ok(), "100바이트 할당 실패");
        let _ = hal.free(handle.unwrap());
    }
}
