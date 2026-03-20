// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! # cpu
//!
//! Linux CPU 스케줄링 직접 제어 HAL 구현.
//!
//! ## 구현 기술
//! - **CPU 사용률**: `/proc/stat` 두 번 읽기 + 시간 델타 계산
//! - **CPU 주파수**: `/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq`
//! - **CPU 모델**:  `/proc/cpuinfo` 파싱 (model name 필드)
//! - **CPU 어피니티**: `sched_setaffinity(2)` + `sched_getaffinity(2)` syscall
//!
//! ## CPU 사용률 계산 알고리즘
//! ```text
//! 1. T1 = /proc/stat 첫 번째 읽기
//! 2. sleep(SAMPLE_INTERVAL_MS)
//! 3. T2 = /proc/stat 두 번째 읽기
//! 4. delta_idle  = T2.idle  - T1.idle
//! 5. delta_total = T2.total - T1.total
//! 6. usage = 1.0 - (delta_idle / delta_total)
//! ```
//!
//! ## 참조
//! - proc(5): https://man7.org/linux/man-pages/man5/proc.5.html
//! - sched_setaffinity(2): https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html

use std::fs;
use std::thread;
use std::time::Duration;

use crate::{CpuState, HalError, HalResponse, ResourceState};

/// CPU 사용률 샘플링 간격 (밀리초).
///
/// 짧을수록 빠르지만 정확도 낮음. 기본 100ms.
const SAMPLE_INTERVAL_MS: u64 = 100;

// ─────────────────────────────────────────────
//  /proc/stat 파싱용 내부 구조체
// ─────────────────────────────────────────────

/// `/proc/stat`의 cpu 라인 파싱 결과.
///
/// 형식: `cpu  <user> <nice> <system> <idle> <iowait> <irq> <softirq> <steal>`
#[derive(Debug, Clone, Copy, Default)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuTimes {
    /// 전체 CPU 시간 (idle 포함) 반환.
    fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    /// idle + iowait 시간 반환.
    ///
    /// iowait도 실질적으로 CPU가 대기 중인 시간.
    fn idle_total(&self) -> u64 {
        self.idle + self.iowait
    }

    /// 두 스냅샷 간의 CPU 사용률 계산 (0.0 ~ 1.0).
    ///
    /// `prev`가 이전 스냅샷, `self`가 이후 스냅샷.
    fn usage_since(&self, prev: &CpuTimes) -> f64 {
        let delta_total = self.total().saturating_sub(prev.total());
        let delta_idle  = self.idle_total().saturating_sub(prev.idle_total());

        if delta_total == 0 {
            return 0.0;
        }
        // 사용률 = 1 - (비사용 시간 / 전체 시간)
        let usage = 1.0 - (delta_idle as f64 / delta_total as f64);
        // 0.0 ~ 1.0 범위 클램프
        usage.clamp(0.0, 1.0)
    }
}

// ─────────────────────────────────────────────
//  LinuxCpuHal 구조체
// ─────────────────────────────────────────────

/// Linux CPU 스케줄링 제어 HAL.
///
/// 상태 비저장(stateless) — 사용률은 매 호출마다 측정.
pub struct LinuxCpuHal;

impl LinuxCpuHal {
    /// 새 LinuxCpuHal 인스턴스 생성.
    pub fn new() -> Self { Self }

    // ─────────────────────────────────────────
    //  공개 인터페이스
    // ─────────────────────────────────────────

    /// 현재 CPU 상태를 측정하여 반환.
    ///
    /// ## 처리 시간
    /// 내부적으로 `SAMPLE_INTERVAL_MS`(100ms)만큼 대기하여
    /// 델타 기반 사용률을 계산함.
    pub fn query_state_inner(&self) -> Result<HalResponse, HalError> {
        let state = Self::read_cpu_state()?;
        Ok(HalResponse::ResourceState(ResourceState::Cpu(state)))
    }

    /// 특정 프로세스를 지정 CPU 코어에 고정(pin).
    ///
    /// ## 알고리즘 (Linux 전용)
    /// 1. cpu_set_t 비트마스크 초기화 → CPU_SET으로 코어 비트 설정
    /// 2. sched_setaffinity(2) syscall 호출
    ///
    /// # 인자
    /// - `pid`: 대상 프로세스 PID (0 = 호출 프로세스)
    /// - `core`: 고정할 CPU 코어 번호 (0-indexed)
    ///
    /// ## 참조
    /// https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html
    ///
    /// **Linux 전용**: 비-Linux 환경에서는 `ResourceNotFound` 반환.
    #[cfg(target_os = "linux")]
    pub fn set_affinity(&self, pid: u32, core: usize) -> Result<(), HalError> {
        // 코어 번호 범위 검증
        let logical_cores = Self::logical_core_count();
        if core >= logical_cores {
            return Err(HalError::InvalidParameter {
                param_name: "core".to_string(),
                message: format!(
                    "코어 번호 {} 초과: 시스템 최대 코어 수는 {} 입니다",
                    core, logical_cores
                ),
            });
        }

        // SAFETY:
        // 1. cpu_set: 스택에 초기화된 지역 변수 — 안전한 메모리 참조
        // 2. CPU_ZERO, CPU_SET: libc가 제공하는 안전한 매크로
        // 3. sched_setaffinity: POSIX syscall, 파라미터 검증됨
        // 4. pid=0이면 호출 프로세스에 적용 (Linux 문서 명시)
        unsafe {
            let mut cpu_set = std::mem::zeroed::<libc::cpu_set_t>();
            libc::CPU_ZERO(&mut cpu_set);
            libc::CPU_SET(core, &mut cpu_set);

            let ret = libc::sched_setaffinity(
                pid as libc::pid_t,
                std::mem::size_of::<libc::cpu_set_t>(),
                &cpu_set,
            );

            if ret != 0 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                return Err(HalError::SyscallFailed {
                    syscall: "sched_setaffinity",
                    errno,
                    message: format!("PID {} 를 코어 {} 에 고정 실패", pid, core),
                });
            }
        }

        Ok(())
    }

    /// 비-Linux 플랫폼 stub: CPU 어피니티 미지원.
    #[cfg(not(target_os = "linux"))]
    pub fn set_affinity(&self, _pid: u32, _core: usize) -> Result<(), HalError> {
        Err(HalError::ResourceNotFound { resource: crate::ResourceType::Cpu })
    }

    /// 특정 프로세스의 현재 CPU 어피니티 마스크 조회.
    ///
    /// 허용된 코어 번호 목록을 반환.
    ///
    /// **Linux 전용**: 비-Linux 환경에서는 `ResourceNotFound` 반환.
    #[cfg(target_os = "linux")]
    pub fn get_affinity(&self, pid: u32) -> Result<Vec<usize>, HalError> {
        // SAFETY:
        // cpu_set: 스택 지역 변수, sched_getaffinity가 채워줌
        let cpu_set = unsafe {
            let mut cpu_set = std::mem::zeroed::<libc::cpu_set_t>();
            let ret = libc::sched_getaffinity(
                pid as libc::pid_t,
                std::mem::size_of::<libc::cpu_set_t>(),
                &mut cpu_set,
            );
            if ret != 0 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                return Err(HalError::SyscallFailed {
                    syscall: "sched_getaffinity",
                    errno,
                    message: format!("PID {} 어피니티 조회 실패", pid),
                });
            }
            cpu_set
        };

        // 비트마스크에서 허용된 코어 번호 추출
        let cores: Vec<usize> = (0..libc::CPU_SETSIZE as usize)
            .filter(|&core| {
                // SAFETY: cpu_set은 sched_getaffinity가 채운 유효한 값
                unsafe { libc::CPU_ISSET(core, &cpu_set) }
            })
            .collect();

        Ok(cores)
    }

    /// 비-Linux 플랫폼 stub: CPU 어피니티 미지원.
    #[cfg(not(target_os = "linux"))]
    pub fn get_affinity(&self, _pid: u32) -> Result<Vec<usize>, HalError> {
        Err(HalError::ResourceNotFound { resource: crate::ResourceType::Cpu })
    }

    // ─────────────────────────────────────────
    //  내부 헬퍼 함수
    // ─────────────────────────────────────────

    /// `/proc/stat` 두 번 읽기 + 델타 계산으로 CPU 상태 반환.
    pub fn read_cpu_state() -> Result<CpuState, HalError> {
        // 1차 스냅샷
        let snap1 = Self::read_proc_stat()?;
        // 100ms 대기 (사용률 델타 계산용)
        thread::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS));
        // 2차 스냅샷
        let snap2 = Self::read_proc_stat()?;

        // 전체 코어 사용률 계산
        let logical_cores = snap2.per_core.len();
        let per_core_usage: Vec<f64> = snap2
            .per_core
            .iter()
            .zip(snap1.per_core.iter())
            .map(|(t2, t1)| t2.usage_since(t1))
            .collect();

        // 평균 사용률 (전체 cpu 라인 기준)
        let total_usage = snap2.total.usage_since(&snap1.total);

        Ok(CpuState {
            logical_cores,
            per_core_usage,
            total_usage,
            frequency_mhz: Self::read_frequency_mhz().unwrap_or(0),
            model_name: Self::read_model_name().unwrap_or_else(|_| "Unknown".to_string()),
        })
    }

    /// `/proc/stat` 한 번 읽기 → 스냅샷 반환.
    ///
    /// ## 파싱 형식
    /// ```text
    /// cpu  10234 0 2341 23452 450 0 12 0 0 0   ← 전체 합계
    /// cpu0 1234  0  230  2934  45 0  1 0 0 0   ← 코어별
    /// cpu1 1234  0  230  2934  45 0  1 0 0 0
    /// ...
    /// ```
    fn read_proc_stat() -> Result<ProcStatSnapshot, HalError> {
        let content = fs::read_to_string("/proc/stat").map_err(|e| HalError::SyscallFailed {
            syscall: "read(/proc/stat)",
            errno: e.raw_os_error().unwrap_or(0),
            message: format!("/proc/stat 읽기 실패: {}", e),
        })?;

        let mut total = CpuTimes::default();
        let mut per_core: Vec<CpuTimes> = Vec::new();

        for line in content.lines() {
            if line.starts_with("cpu ") || line.starts_with("cpu\t") {
                // 전체 합계 라인
                total = Self::parse_cpu_line(line)?;
            } else if line.starts_with("cpu") {
                // 개별 코어 라인 (cpu0, cpu1, ...)
                per_core.push(Self::parse_cpu_line(line)?);
            }
        }

        // 개별 코어 정보가 없으면 전체 1코어로 처리
        if per_core.is_empty() {
            per_core.push(total);
        }

        Ok(ProcStatSnapshot { total, per_core })
    }

    /// `/proc/stat` 의 cpu 라인 파싱.
    ///
    /// 형식: `cpuN user nice system idle iowait irq softirq steal ...`
    fn parse_cpu_line(line: &str) -> Result<CpuTimes, HalError> {
        // "cpu " 또는 "cpu0 " 접두사 제거 후 숫자 파싱
        let values: Vec<u64> = line
            .split_whitespace()
            .skip(1) // "cpu" 또는 "cpu0" 건너뜀
            .take(8) // user nice system idle iowait irq softirq steal
            .map(|s| s.parse::<u64>().unwrap_or(0))
            .collect();

        if values.len() < 4 {
            return Err(HalError::InternalError(format!(
                "/proc/stat 파싱 실패: 필드 부족 — '{}'",
                line
            )));
        }

        Ok(CpuTimes {
            user:    values[0],
            nice:    values[1],
            system:  values[2],
            idle:    values[3],
            iowait:  values.get(4).copied().unwrap_or(0),
            irq:     values.get(5).copied().unwrap_or(0),
            softirq: values.get(6).copied().unwrap_or(0),
            steal:   values.get(7).copied().unwrap_or(0),
        })
    }

    /// CPU 현재 주파수 (MHz) 반환.
    ///
    /// 경로: `/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq`
    /// 단위: kHz → MHz 변환
    fn read_frequency_mhz() -> Result<u64, HalError> {
        // scaling_cur_freq: 현재 실제 동작 주파수 (kHz 단위)
        let content = fs::read_to_string(
            "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
        )
        .or_else(|_| {
            // cpufreq 없는 환경 (VM, WSL 등): cpuinfo_cur_freq 시도
            fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_cur_freq")
        })
        .map_err(|_e| HalError::ResourceUnavailable {
            resource: crate::ResourceType::Cpu,
        })?;

        // kHz → MHz 변환
        let khz: u64 = content.trim().parse().unwrap_or(0);
        Ok(khz / 1000)
    }

    /// CPU 모델 이름 반환.
    ///
    /// `/proc/cpuinfo` 에서 `model name` 필드 추출.
    fn read_model_name() -> Result<String, HalError> {
        let content = fs::read_to_string("/proc/cpuinfo").map_err(|e| HalError::SyscallFailed {
            syscall: "read(/proc/cpuinfo)",
            errno: e.raw_os_error().unwrap_or(0),
            message: e.to_string(),
        })?;

        // "model name	: Intel(R) Core(TM) i7-..." 형식 파싱
        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some(name) = line.splitn(2, ':').nth(1) {
                    return Ok(name.trim().to_string());
                }
            }
        }

        Ok("Unknown CPU".to_string())
    }

    /// 시스템 논리 코어 수 반환.
    ///
    /// `/proc/stat`의 cpuN 라인 수로 결정.
    /// `set_affinity` (Linux 전용)에서 코어 범위 검증에 사용.
    #[cfg(target_os = "linux")]
    fn logical_core_count() -> usize {
        fs::read_to_string("/proc/stat")
            .unwrap_or_default()
            .lines()
            .filter(|l| {
                l.starts_with("cpu")
                    && l.len() > 3
                    && l.chars().nth(3).map(|c| c.is_ascii_digit()).unwrap_or(false)
            })
            .count()
            .max(1) // 최소 1코어
    }
}

impl Default for LinuxCpuHal {
    fn default() -> Self { Self::new() }
}

/// `/proc/stat` 단일 스냅샷.
struct ProcStatSnapshot {
    /// 전체 CPU 합계 라인
    total: CpuTimes,
    /// 코어별 라인
    per_core: Vec<CpuTimes>,
}

// ─────────────────────────────────────────────
//  단위 테스트
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── /proc/stat 파싱 테스트 ─────────────────

    #[test]
    fn test_read_proc_stat_success() {
        // /proc/stat 읽기 및 파싱 성공 확인
        let snap = LinuxCpuHal::read_proc_stat();
        assert!(snap.is_ok(), "/proc/stat 파싱 실패: {:?}", snap.err());

        let s = snap.unwrap();
        assert!(!s.per_core.is_empty(), "코어 정보 없음");
        assert!(s.total.total() > 0, "전체 CPU 시간이 0입니다");
    }

    #[test]
    fn test_parse_cpu_line_valid() {
        // 정상 cpu 라인 파싱 검증
        let line = "cpu  2255 34 2290 22625563 6290 127 456 0 0 0";
        let result = LinuxCpuHal::parse_cpu_line(line);
        assert!(result.is_ok());

        let times = result.unwrap();
        assert_eq!(times.user, 2255);
        assert_eq!(times.nice, 34);
        assert_eq!(times.system, 2290);
        assert_eq!(times.idle, 22625563);
        assert_eq!(times.iowait, 6290);
    }

    #[test]
    fn test_cpu_times_usage_calculation() {
        // 델타 기반 사용률 계산 검증
        let prev = CpuTimes {
            user: 100, nice: 0, system: 50, idle: 850,
            iowait: 0, irq: 0, softirq: 0, steal: 0,
        };
        let curr = CpuTimes {
            user: 200, nice: 0, system: 100, idle: 900,
            iowait: 0, irq: 0, softirq: 0, steal: 0,
        };
        // delta_total = 350, delta_idle = 50
        // usage = 1 - (50/350) ≈ 0.857
        let usage = curr.usage_since(&prev);
        let expected = 1.0 - (50.0 / 350.0);
        assert!((usage - expected).abs() < 1e-9, "예상: {}, 실제: {}", expected, usage);
    }

    #[test]
    fn test_cpu_times_usage_zero_delta() {
        // 델타가 0인 경우 0.0 반환 (0 나누기 방지)
        let same = CpuTimes::default();
        assert_eq!(same.usage_since(&same), 0.0);
    }

    #[test]
    fn test_cpu_usage_clamp_0_to_1() {
        // 사용률은 항상 0.0 ~ 1.0 범위 내여야 함
        let prev = CpuTimes { user: 0, nice: 0, system: 0, idle: 1000, ..Default::default() };
        let curr = CpuTimes { user: 1000, nice: 0, system: 0, idle: 1000, ..Default::default() };
        let usage = curr.usage_since(&prev);
        assert!(usage >= 0.0 && usage <= 1.0);
    }

    // ── CPU 상태 조회 통합 테스트 ──────────────

    #[test]
    fn test_read_cpu_state_valid() {
        // 실제 CPU 상태 조회 (100ms 소요)
        let result = LinuxCpuHal::read_cpu_state();
        assert!(result.is_ok(), "CPU 상태 조회 실패: {:?}", result.err());

        let state = result.unwrap();
        assert!(state.logical_cores >= 1);
        assert_eq!(state.per_core_usage.len(), state.logical_cores);
        // 모든 코어 사용률은 0.0 ~ 1.0
        for (i, &usage) in state.per_core_usage.iter().enumerate() {
            assert!(
                usage >= 0.0 && usage <= 1.0,
                "코어 {} 사용률 범위 초과: {}",
                i, usage
            );
        }
        assert!(state.total_usage >= 0.0 && state.total_usage <= 1.0);
    }

    // ── sched_setaffinity 테스트 ───────────────

    #[test]
    fn test_get_affinity_self() {
        // 현재 프로세스(pid=0) 어피니티 조회 검증
        let hal = LinuxCpuHal::new();
        let result = hal.get_affinity(0);
        // 권한 없을 수 있으니 에러도 OK (CI 환경)
        match result {
            Ok(cores) => {
                assert!(!cores.is_empty(), "허용된 코어가 없습니다");
            }
            Err(HalError::SyscallFailed { .. }) => {
                // CI 환경에서 권한 없을 경우 허용
            }
            Err(e) => panic!("예상치 못한 에러: {:?}", e),
        }
    }

    #[test]
    fn test_set_affinity_invalid_core() {
        // 존재하지 않는 코어 번호로 set_affinity → InvalidParameter
        let hal = LinuxCpuHal::new();
        // 매우 큰 코어 번호: 절대 존재하지 않음
        let result = hal.set_affinity(0, usize::MAX);
        assert!(matches!(result, Err(HalError::InvalidParameter { .. })));
    }

    // ── 모델 이름 / 주파수 테스트 ─────────────

    #[test]
    fn test_read_model_name_not_empty() {
        // CPU 모델 이름이 비어 있지 않은지 확인
        let name = LinuxCpuHal::read_model_name().unwrap_or_else(|_| "Unknown".to_string());
        assert!(!name.is_empty());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_logical_core_count_at_least_one() {
        // 논리 코어 수는 항상 1 이상 (Linux 전용: /proc/stat 기반)
        let cores = LinuxCpuHal::logical_core_count();
        assert!(cores >= 1, "코어 수가 0입니다");
    }
}
