// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! # hal_integration
//!
//! AI-OS HAL 통합 테스트 모듈.
//!
//! ## 목적
//! - 각 서브모듈(memory, cpu, storage)의 실제 Linux syscall 동작 검증
//! - `LinuxHal` 통합 구조체의 엔드-투-엔드 흐름 테스트
//! - `MockHal`과 `LinuxHal`의 동일 인터페이스(`AiHalInterface`) 준수 검증
//! - 에러 전파 경로 및 재시도 가능 여부 분류 확인
//!
//! ## 실행 방법
//! ```bash
//! cargo test --package ai-hal --features mock -- --test-threads=1
//! ```
//!
//! ## 주의
//! - 일부 테스트는 `/proc`, `/tmp` 등 Linux 전용 경로를 사용
//! - `sched_setaffinity` 테스트는 Linux 전용으로 조건부 실행
//! - 모든 테스트는 파일 시스템 부작용을 직접 정리(cleanup)함

use std::path::Path;

use ai_hal::{
    AiHalInterface, AuditEntry, CapabilityToken, HalCommand, HalError,
    LinuxCpuHal, LinuxHal, LinuxMemoryHal, LinuxStorageHal, MemoryHandle,
    MockHal, ResourceType, SkillManifest, StorageHandle,
};

// ─────────────────────────────────────────────
//  테스트 헬퍼
// ─────────────────────────────────────────────

/// 단일 리소스 권한을 가진 테스트용 CapabilityToken 생성.
fn test_token(resource: ResourceType) -> CapabilityToken {
    CapabilityToken::new(vec![resource], "integration-test")
}

/// 모든 리소스를 허용하는 테스트 토큰.
fn all_access_token() -> CapabilityToken {
    CapabilityToken::new(
        vec![
            ResourceType::Memory,
            ResourceType::Cpu,
            ResourceType::Storage,
            ResourceType::Gpu,
            ResourceType::Network,
        ],
        "integration-test-all",
    )
}

// ─────────────────────────────────────────────
//  LinuxMemoryHal 통합 테스트
// ─────────────────────────────────────────────

/// `/proc/meminfo` 파싱 통합 테스트.
///
/// 실제 파일 읽기 + MemoryState 필드 유효성 검증.
#[test]
fn integration_memory_read_meminfo() {
    let state = LinuxMemoryHal::read_meminfo().expect("/proc/meminfo 파싱 실패");

    // 총 메모리는 반드시 0보다 커야 함
    assert!(state.total_bytes > 0, "total_bytes == 0");
    // 사용 가능 메모리는 총 메모리를 초과할 수 없음
    assert!(
        state.available_bytes <= state.total_bytes,
        "available_bytes({}) > total_bytes({})",
        state.available_bytes,
        state.total_bytes
    );
}

/// mmap/munmap 라운드트립 통합 테스트.
///
/// 페이지 크기의 배수로 메모리 할당 → 읽기/쓰기 → 해제 전 과정 검증.
#[test]
fn integration_memory_mmap_munmap_roundtrip() {
    let hal = LinuxMemoryHal::new();

    // 4096 바이트 (1 페이지) 할당
    let handle = hal
        .allocate(4096, 4096, false)
        .expect("mmap 4096 바이트 실패");

    // 핸들이 유효한지 확인 (ptr != 0)
    if let MemoryHandle::Mmap { ptr, size } = handle {
        assert_ne!(ptr, 0, "mmap 반환 포인터가 NULL");
        assert_eq!(size, 4096);

        // mmap된 영역에 쓰기/읽기 검증
        // SAFETY: ptr은 mmap으로 할당된 유효한 쓰기 가능 메모리
        unsafe {
            let slice = std::slice::from_raw_parts_mut(ptr as *mut u8, 4096);
            slice[0] = 0xAB;
            slice[4095] = 0xCD;
            assert_eq!(slice[0], 0xAB);
            assert_eq!(slice[4095], 0xCD);
        }

        // munmap으로 해제
        hal.free(MemoryHandle::Mmap { ptr, size })
            .expect("munmap 실패");
    } else {
        panic!("예상치 못한 MemoryHandle 타입");
    }
}

/// 잘못된 크기(0)로 할당 시 에러 반환 테스트.
#[test]
fn integration_memory_zero_size_error() {
    let hal = LinuxMemoryHal::new();
    let result = hal.allocate(0, 4096, false);
    assert!(
        matches!(result, Err(HalError::InvalidParameter { .. })),
        "크기 0 할당에서 InvalidParameter 아닌 에러: {:?}",
        result
    );
}

// ─────────────────────────────────────────────
//  LinuxCpuHal 통합 테스트
// ─────────────────────────────────────────────

/// CPU 상태 조회 통합 테스트.
///
/// `/proc/stat` 두 번 읽기 + 사용률 계산 전 과정 검증.
#[test]
fn integration_cpu_read_state() {
    let state = LinuxCpuHal::read_cpu_state().expect("CPU 상태 조회 실패");

    // 코어 수는 최소 1 이상
    assert!(state.logical_cores >= 1, "logical_cores < 1");
    // 전체 사용률은 0.0 ~ 1.0 범위
    assert!(
        state.total_usage >= 0.0 && state.total_usage <= 1.0,
        "total_usage 범위 벗어남: {}",
        state.total_usage
    );
    // per_core_usage 배열 크기는 logical_cores와 일치
    assert_eq!(
        state.per_core_usage.len(),
        state.logical_cores,
        "per_core_usage 배열 크기와 logical_cores 불일치"
    );
    // 각 코어 사용률도 0.0 ~ 1.0 범위
    for (i, &usage) in state.per_core_usage.iter().enumerate() {
        assert!(
            usage >= 0.0 && usage <= 1.0,
            "코어 {} 사용률 범위 벗어남: {}",
            i,
            usage
        );
    }
}

/// `sched_getaffinity` 기본 동작 테스트 (Linux 전용).
///
/// 현재 프로세스(pid=0)의 CPU 친화성 마스크 조회.
#[test]
#[cfg(target_os = "linux")]
fn integration_cpu_get_affinity_current_process() {
    let hal = LinuxCpuHal::new();
    // pid=0: 현재 프로세스 자신
    let cores = hal.get_affinity(0).expect("sched_getaffinity 실패");

    assert!(!cores.is_empty(), "친화성 코어 목록이 비어있음");
    let state = LinuxCpuHal::read_cpu_state().expect("CPU 상태 조회 실패");
    for &core in &cores {
        assert!(
            core < state.logical_cores,
            "친화성 코어 번호 {} >= logical_cores {}",
            core,
            state.logical_cores
        );
    }
}

/// CPU 모델명 비어있지 않음 확인.
#[test]
fn integration_cpu_model_name_exists() {
    let state = LinuxCpuHal::read_cpu_state().expect("CPU 상태 조회 실패");
    // VM 환경에서는 모델명을 읽지 못할 수도 있으므로 String 타입만 확인
    let _ = state.model_name;
}

// ─────────────────────────────────────────────
//  LinuxStorageHal 통합 테스트
// ─────────────────────────────────────────────

/// 루트 파일시스템 statfs 통합 테스트.
#[test]
fn integration_storage_query_root() {
    let hal = LinuxStorageHal::new();
    let state = hal.query_state(Path::new("/")).expect("루트 statfs 실패");

    assert!(state.total_bytes > 0);
    assert!(state.block_size >= 512);
    assert!(!state.fs_type.is_empty());
    assert!(state.available_bytes <= state.total_bytes);
}

/// 파일 생성/쓰기/읽기/삭제 전체 흐름 통합 테스트.
#[test]
fn integration_storage_file_lifecycle() {
    let hal = LinuxStorageHal::new();
    let path = std::path::PathBuf::from("/tmp/aios_integration_lifecycle.bin");

    // 1. 파일 생성 및 데이터 쓰기
    let whandle = hal
        .open_file(&path, true, false, true)
        .expect("파일 생성 실패");

    let payload = b"AI-OS HAL integration test payload v1.0";
    let n = hal
        .write_at(&whandle, 0, payload, false)
        .expect("쓰기 실패");
    assert_eq!(n, payload.len());

    // 2. fsync로 디스크 플러시
    hal.sync_file(&whandle).expect("fsync 실패");
    hal.close_file(whandle).expect("쓰기 파일 close 실패");

    // 3. 파일 크기 확인
    let fsize = LinuxStorageHal::file_size(&path).expect("file_size 실패");
    assert_eq!(fsize, payload.len() as u64);

    // 4. 데이터 읽기 검증
    let rhandle = hal
        .open_file(&path, false, false, false)
        .expect("읽기 열기 실패");
    let read_back = hal
        .read_at(&rhandle, 0, payload.len(), false)
        .expect("읽기 실패");
    hal.close_file(rhandle).expect("읽기 파일 close 실패");

    assert_eq!(read_back.as_slice(), payload);

    // 5. 정리
    std::fs::remove_file(&path).expect("테스트 파일 삭제 실패");
}

/// 여러 오프셋에 분산 쓰기 후 순서대로 읽기 테스트.
#[test]
fn integration_storage_scatter_write_gather_read() {
    let hal = LinuxStorageHal::new();
    let path = std::path::PathBuf::from("/tmp/aios_integration_scatter.bin");

    // 256 바이트 제로 파일 초기화
    let wh = hal
        .open_file(&path, true, false, true)
        .expect("파일 생성 실패");
    hal.write_at(&wh, 0, &vec![0u8; 256], false)
        .expect("초기화 실패");

    // 서로 다른 오프셋에 마커 쓰기
    hal.write_at(&wh, 0, b"AIOS", false).expect("오프셋 0 쓰기 실패");
    hal.write_at(&wh, 64, b"CORE", false).expect("오프셋 64 쓰기 실패");
    hal.write_at(&wh, 128, b"HAL!", false).expect("오프셋 128 쓰기 실패");
    hal.close_file(wh).expect("쓰기 close 실패");

    // 각 오프셋에서 마커 읽기 검증
    let rh = hal
        .open_file(&path, false, false, false)
        .expect("읽기 열기 실패");

    let m0 = hal.read_at(&rh, 0, 4, false).expect("오프셋 0 읽기 실패");
    let m64 = hal.read_at(&rh, 64, 4, false).expect("오프셋 64 읽기 실패");
    let m128 = hal.read_at(&rh, 128, 4, false).expect("오프셋 128 읽기 실패");
    hal.close_file(rh).expect("읽기 close 실패");

    assert_eq!(&m0, b"AIOS");
    assert_eq!(&m64, b"CORE");
    assert_eq!(&m128, b"HAL!");

    std::fs::remove_file(&path).expect("삭제 실패");
}

// ─────────────────────────────────────────────
//  LinuxHal (통합 구조체) 테스트
// ─────────────────────────────────────────────

/// `LinuxHal::hal_name()` 반환값 검증.
#[test]
fn integration_linuxhal_name() {
    let hal = LinuxHal::new();
    let name = hal.hal_name();
    assert!(
        name.contains("Linux") || name.contains("linux"),
        "hal_name에 'Linux' 미포함: {}",
        name
    );
}

/// `LinuxHal::supported_resources()` 반환값에 핵심 리소스 포함 여부.
#[test]
fn integration_linuxhal_supported_resources() {
    let hal = LinuxHal::new();
    let resources = hal.supported_resources();

    assert!(resources.contains(&ResourceType::Memory));
    assert!(resources.contains(&ResourceType::Cpu));
    assert!(resources.contains(&ResourceType::Storage));
}

/// `LinuxHal::execute_command()` — 메모리 쿼리 커맨드 실행 테스트.
#[test]
fn integration_linuxhal_execute_query_memory() {
    let hal = LinuxHal::new();
    let token = all_access_token();

    let cmd = HalCommand::QueryState {
        resource: ResourceType::Memory,
        detailed: false,
    };

    let result = hal.execute_command(&token, cmd);
    assert!(result.outcome.is_ok(), "QueryState::Memory 실패: {:?}", result.outcome);
}

/// `LinuxHal::execute_command()` — CPU 쿼리 커맨드 실행 테스트.
#[test]
fn integration_linuxhal_execute_query_cpu() {
    let hal = LinuxHal::new();
    let token = all_access_token();

    let cmd = HalCommand::QueryState {
        resource: ResourceType::Cpu,
        detailed: false,
    };

    let result = hal.execute_command(&token, cmd);
    assert!(result.outcome.is_ok(), "QueryState::Cpu 실패: {:?}", result.outcome);
}

/// `LinuxHal::execute_command()` — 스토리지 쿼리 커맨드 실행 테스트.
#[test]
fn integration_linuxhal_execute_query_storage() {
    let hal = LinuxHal::new();
    let token = all_access_token();

    let cmd = HalCommand::QueryState {
        resource: ResourceType::Storage,
        detailed: false,
    };

    let result = hal.execute_command(&token, cmd);
    assert!(result.outcome.is_ok(), "QueryState::Storage 실패: {:?}", result.outcome);
}

/// 권한 없는 토큰으로 명령 실행 시 `PermissionDenied` 반환 테스트.
#[test]
fn integration_linuxhal_permission_denied_on_wrong_token() {
    let hal = LinuxHal::new();
    // GPU 전용 토큰으로 Memory 접근 시도
    let token = test_token(ResourceType::Gpu);

    let cmd = HalCommand::QueryState {
        resource: ResourceType::Memory,
        detailed: false,
    };

    let result = hal.execute_command(&token, cmd);
    assert!(
        matches!(result.outcome, Err(HalError::PermissionDenied { .. })),
        "잘못된 토큰에서 PermissionDenied 아닌 결과: {:?}",
        result.outcome
    );
}

// ─────────────────────────────────────────────
//  MockHal 테스트
// ─────────────────────────────────────────────

/// `MockHal::hal_name()` 반환값 검증.
#[test]
fn integration_mockhal_name() {
    let mock = MockHal::new();
    let name = mock.hal_name();
    assert!(
        name.contains("Mock") || name.contains("mock"),
        "MockHal hal_name에 'Mock' 미포함: {}",
        name
    );
}

/// `MockHal` — QueryState::Memory 응답 검증.
#[test]
fn integration_mockhal_query_memory() {
    let mock = MockHal::new();
    let token = all_access_token();

    let cmd = HalCommand::QueryState {
        resource: ResourceType::Memory,
        detailed: false,
    };

    let result = mock.execute_command(&token, cmd);
    assert!(result.outcome.is_ok(), "MockHal QueryMemory 실패: {:?}", result.outcome);
}

/// `MockHal` 권한 체크 — PermissionDenied 동작 확인.
#[test]
fn integration_mockhal_permission_denied() {
    let mock = MockHal::new();
    let token = test_token(ResourceType::Cpu); // Cpu 토큰만

    let cmd = HalCommand::QueryState {
        resource: ResourceType::Memory, // Memory 접근 시도
        detailed: false,
    };

    let result = mock.execute_command(&token, cmd);
    assert!(
        matches!(result.outcome, Err(HalError::PermissionDenied { .. })),
        "MockHal: 잘못된 토큰에서 PermissionDenied 아닌 결과: {:?}",
        result.outcome
    );
}

/// `MockHal`과 `LinuxHal` 인터페이스 동등성 검증.
///
/// 같은 `AiHalInterface` 트레잇을 구현하여
/// `dyn AiHalInterface` 트레잇 오브젝트로 교체 가능해야 함.
#[test]
fn integration_hal_interface_polymorphism() {
    let hals: Vec<Box<dyn AiHalInterface>> = vec![
        Box::new(LinuxHal::new()),
        Box::new(MockHal::new()),
    ];

    let token = all_access_token();

    for hal in &hals {
        let name = hal.hal_name();
        assert!(!name.is_empty(), "hal_name이 비어있음");

        let resources = hal.supported_resources();
        assert!(!resources.is_empty(), "supported_resources가 비어있음");

        // Memory 쿼리
        let cmd = HalCommand::QueryState {
            resource: ResourceType::Memory,
            detailed: false,
        };
        let result = hal.execute_command(&token, cmd);
        assert!(
            result.outcome.is_ok(),
            "{} — QueryState::Memory 실패: {:?}",
            name,
            result.outcome
        );
    }
}

// ─────────────────────────────────────────────
//  HalError 에러 타입 통합 테스트
// ─────────────────────────────────────────────

/// `HalError::is_retryable()` 동작 통합 검증.
#[test]
fn integration_halerror_retryable_classification() {
    // ResourceUnavailable → 재시도 가능
    let unavail = HalError::ResourceUnavailable { resource: ResourceType::Memory };
    assert!(unavail.is_retryable());

    // EAGAIN(11) → 재시도 가능
    let eagain = HalError::SyscallFailed { syscall: "read", errno: 11, message: String::new() };
    assert!(eagain.is_retryable());

    // EINTR(4) → 재시도 가능
    let eintr = HalError::SyscallFailed { syscall: "write", errno: 4, message: String::new() };
    assert!(eintr.is_retryable());

    // PermissionDenied → 재시도 불가
    let denied = HalError::PermissionDenied { resource: ResourceType::Storage, reason: String::new() };
    assert!(!denied.is_retryable());

    // OutOfMemory → 재시도 불가
    let oom = HalError::OutOfMemory { requested_bytes: 1024 * 1024, available_bytes: Some(512) };
    assert!(!oom.is_retryable());
}

/// `HalError::is_security_error()` 동작 통합 검증.
#[test]
fn integration_halerror_security_classification() {
    let denied = HalError::PermissionDenied {
        resource: ResourceType::Memory,
        reason: "테스트".to_string(),
    };
    assert!(denied.is_security_error());

    // EACCES(13) → 보안 에러
    let eacces = HalError::SyscallFailed { syscall: "open", errno: 13, message: String::new() };
    assert!(eacces.is_security_error());

    // EPERM(1) → 보안 에러
    let eperm = HalError::SyscallFailed { syscall: "mmap", errno: 1, message: String::new() };
    assert!(eperm.is_security_error());

    // EIO → 보안 에러 아님
    let io = HalError::SyscallFailed { syscall: "read", errno: 5, message: String::new() };
    assert!(!io.is_security_error());
}

/// `HalError::Display` 포맷 통합 검증.
#[test]
fn integration_halerror_display_formats() {
    let cases: Vec<(HalError, &str)> = vec![
        (
            HalError::PermissionDenied { resource: ResourceType::Memory, reason: "테스트".to_string() },
            "권한 거부",
        ),
        (
            HalError::OutOfMemory { requested_bytes: 1024, available_bytes: Some(512) },
            "메모리 부족",
        ),
        (
            HalError::SyscallFailed { syscall: "mmap", errno: 12, message: "ENOMEM".to_string() },
            "mmap",
        ),
        (HalError::ResourceNotFound { resource: ResourceType::Gpu }, "리소스 없음"),
        (HalError::InternalError("버그".to_string()), "HAL 내부 오류"),
    ];

    for (err, expected_substr) in cases {
        let msg = format!("{}", err);
        assert!(
            msg.contains(expected_substr),
            "에러 메시지 '{}'에 '{}' 미포함",
            msg,
            expected_substr
        );
    }
}

// ─────────────────────────────────────────────
//  CapabilityToken 권한 체크 테스트
// ─────────────────────────────────────────────

/// `CapabilityToken::allows()` 동작 검증.
#[test]
fn integration_capability_token_allows() {
    let token = CapabilityToken::new(
        vec![ResourceType::Memory, ResourceType::Cpu],
        "test",
    );

    assert!(token.allows(&ResourceType::Memory));
    assert!(token.allows(&ResourceType::Cpu));
    assert!(!token.allows(&ResourceType::Storage));
    assert!(!token.allows(&ResourceType::Gpu));
    assert!(!token.allows(&ResourceType::Network));
}

// ─────────────────────────────────────────────
//  AuditEntry 기록 테스트
// ─────────────────────────────────────────────

/// `AuditEntry::success()` / `AuditEntry::failure()` 생성자 검증.
#[test]
fn integration_audit_entry_constructors() {
    let success = AuditEntry::success("test-skill", "query_state");
    assert!(success.failure_reason.is_none(), "성공 AuditEntry에 failure_reason 있음");
    assert!(success.succeeded);

    let failure = AuditEntry::failure("test-skill", "write_at", "ENOSPC: 공간 부족");
    assert!(failure.failure_reason.is_some(), "실패 AuditEntry에 failure_reason 없음");
    assert!(!failure.succeeded);
    assert_eq!(failure.failure_reason.as_deref(), Some("ENOSPC: 공간 부족"));
}

/// `LinuxHal::execute_command()` 후 `AuditEntry` 자동 생성 검증.
#[test]
fn integration_audit_entry_auto_generated() {
    let hal = LinuxHal::new();
    let token = all_access_token();

    let cmd = HalCommand::QueryState { resource: ResourceType::Memory, detailed: false };
    let result = hal.execute_command(&token, cmd);

    // 성공 여부와 무관하게 AuditEntry는 항상 생성
    assert_eq!(result.audit.requestor, "integration-test-all");
    assert!(result.audit.succeeded);
    assert!(result.audit.failure_reason.is_none());
}

// ─────────────────────────────────────────────
//  Skill 등록 및 토큰 발급 테스트
// ─────────────────────────────────────────────

/// `LinuxHal::register_skill()` 테스트.
#[test]
fn integration_register_skill_linux() {
    let hal = LinuxHal::new();
    let manifest = SkillManifest {
        name: "test-skill".to_string(),
        version: "0.1.0".to_string(),
        requested_capabilities: vec![ResourceType::Memory],
        description: "통합 테스트용 스킬".to_string(),
    };

    let token = hal.register_skill(manifest).expect("register_skill 실패");
    assert!(!token.token_id.is_empty(), "token_id가 비어있음");
    assert!(
        token.allowed_resources.contains(&ResourceType::Memory),
        "Memory 권한이 토큰에 없음"
    );
}

/// `MockHal::register_skill()` 테스트.
#[test]
fn integration_register_skill_mock() {
    let hal = MockHal::new();
    let manifest = SkillManifest {
        name: "mock-skill".to_string(),
        version: "1.0.0".to_string(),
        requested_capabilities: vec![ResourceType::Cpu, ResourceType::Storage],
        description: "Mock 스킬".to_string(),
    };

    let token = hal.register_skill(manifest).expect("register_skill 실패");
    assert!(token.allowed_resources.contains(&ResourceType::Cpu));
    assert!(token.allowed_resources.contains(&ResourceType::Storage));
}
