// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! # ai-hal
//!
//! AI-OS Hardware Abstraction Layer (HAL) 핵심 크레이트.
//!
//! ## 설계 원칙
//! - AI Core가 앱 없이 하드웨어를 직접 제어하는 추상 인터페이스 제공
//! - capability-based 권한 모델: 토큰 없이는 어떤 HAL 연산도 불가
//! - 모든 HAL 연산은 감사 로그(audit log)에 기록됨
//!
//! ## 아키텍처 위치
//! ```
//! AI Core (Python) → ai-core-bridge → ai-hal (이 크레이트) → Linux Kernel
//! ```

#![forbid(unsafe_code)] // 안전하지 않은 코드 금지 (명시적 unsafe 블록만 허용)
#![warn(missing_docs, clippy::all)]

use std::fmt;
use std::time::SystemTime;

// ─────────────────────────────────────────────
//  섹션 1: 리소스 타입 정의
// ─────────────────────────────────────────────

/// HAL이 제어할 수 있는 하드웨어 리소스 종류.
///
/// M0에서는 Memory / Cpu / Storage 구현,
/// M2 이후 Gpu / Audio / Camera / Network 추가 예정.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive] // 미래 변형 추가를 위해 non_exhaustive 사용
pub enum ResourceType {
    /// 시스템 메인 메모리 (RAM)
    Memory,
    /// CPU 코어 및 스케줄링
    Cpu,
    /// 영구 저장장치 (HDD/SSD/NVMe)
    Storage,
    /// 그래픽 처리 장치 (M2 이후 구현)
    Gpu,
    /// 오디오 입출력 (M2 이후 구현)
    Audio,
    /// 카메라 입력 (M2 이후 구현)
    Camera,
    /// 네트워크 인터페이스
    Network,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Memory => "Memory",
            Self::Cpu => "CPU",
            Self::Storage => "Storage",
            Self::Gpu => "GPU",
            Self::Audio => "Audio",
            Self::Camera => "Camera",
            Self::Network => "Network",
        };
        write!(f, "{}", name)
    }
}

// ─────────────────────────────────────────────
//  섹션 2: HAL 명령 정의
// ─────────────────────────────────────────────

/// AI Core → HAL로 전달되는 명령 열거형.
///
/// 모든 명령은 `CapabilityToken`을 동반해야 실행됨.
/// 설계 참조: capability-based security (Saltzer & Schroeder, 1975)
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HalCommand {
    /// 지정 리소스의 현재 상태를 조회
    QueryState {
        /// 조회할 리소스 종류
        resource: ResourceType,
        /// 상세 조회 여부 (false = 요약, true = 전체)
        detailed: bool,
    },

    /// 메모리 영역 할당 요청
    AllocateMemory {
        /// 요청 바이트 수
        size_bytes: usize,
        /// 메모리 정렬 요구사항 (2의 거듭제곱)
        alignment: usize,
        /// 이 할당이 공유 가능한지 여부
        shared: bool,
    },

    /// 이전에 할당된 메모리 해제
    FreeMemory {
        /// 해제할 메모리 핸들 (AllocateMemory 결과로 받은 ID)
        handle: MemoryHandle,
    },

    /// CPU 스케줄링 힌트 제공 (강제 아님, OS 권고)
    CpuSchedulingHint {
        /// 대상 태스크 식별자
        task_id: u64,
        /// 우선순위 레벨 (0 = 최저, 255 = 최고)
        priority: u8,
        /// 선호 CPU 코어 번호 (None이면 OS 결정)
        preferred_core: Option<usize>,
    },

    /// 스토리지 경로에 대한 읽기 스트림 열기
    OpenStorageRead {
        /// 접근할 경로
        path: std::path::PathBuf,
    },

    /// 스토리지 경로에 대한 쓰기 스트림 열기
    OpenStorageWrite {
        /// 접근할 경로
        path: std::path::PathBuf,
        /// 파일이 없으면 생성할지 여부
        create_if_missing: bool,
    },

    /// Skill을 HAL 런타임에 등록
    RegisterSkill {
        /// Skill 메타데이터
        manifest: SkillManifest,
    },
}

// ─────────────────────────────────────────────
//  섹션 3: HAL 응답 / 상태 타입
// ─────────────────────────────────────────────

/// HAL 명령 실행 결과.
///
/// 성공 시 `HalResult::Ok(HalResponse)`, 실패 시 `HalResult::Err(HalError)`.
/// 표준 `Result<T, E>` 대신 자체 타입을 사용하는 이유:
/// 감사 로그에 항상 결과가 기록되어야 하기 때문.
#[derive(Debug)]
pub struct HalResult {
    /// 실행된 명령의 감사 엔트리
    pub audit: AuditEntry,
    /// 실제 실행 결과
    pub outcome: Result<HalResponse, HalError>,
}

/// HAL 명령 성공 응답 종류.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HalResponse {
    /// 상태 조회 응답
    ResourceState(ResourceState),
    /// 메모리 할당 성공 응답
    MemoryAllocated(MemoryHandle),
    /// 스토리지 스트림 핸들
    StorageHandle(StorageHandle),
    /// Skill 등록 성공 토큰
    SkillRegistered(SkillToken),
    /// 응답 데이터가 없는 성공 (e.g. FreeMemory)
    Ok,
}

/// 리소스 상태 스냅샷.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ResourceState {
    /// 메모리 상태
    Memory(MemoryState),
    /// CPU 상태
    Cpu(CpuState),
    /// 스토리지 상태
    Storage(StorageState),
}

/// 메모리 상태 스냅샷.
#[derive(Debug, Clone)]
pub struct MemoryState {
    /// 총 물리 메모리 (바이트)
    pub total_bytes: u64,
    /// 현재 사용 중인 메모리 (바이트)
    pub used_bytes: u64,
    /// 사용 가능한 메모리 (바이트)
    pub free_bytes: u64,
    /// 페이지 크기 (바이트)
    pub page_size: usize,
}

impl MemoryState {
    /// 메모리 사용률을 0.0 ~ 1.0 범위로 반환.
    pub fn usage_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.used_bytes as f64 / self.total_bytes as f64
    }
}

/// CPU 상태 스냅샷.
#[derive(Debug, Clone)]
pub struct CpuState {
    /// 논리 코어 수
    pub logical_cores: usize,
    /// 각 코어별 사용률 (0.0 ~ 1.0)
    pub per_core_usage: Vec<f64>,
    /// 현재 CPU 주파수 (MHz)
    pub frequency_mhz: u64,
}

/// 스토리지 상태 스냅샷.
#[derive(Debug, Clone)]
pub struct StorageState {
    /// 총 용량 (바이트)
    pub total_bytes: u64,
    /// 사용 중인 용량 (바이트)
    pub used_bytes: u64,
    /// 마운트 포인트 경로
    pub mount_point: std::path::PathBuf,
}

// ─────────────────────────────────────────────
//  섹션 4: 핸들 및 토큰 타입
// ─────────────────────────────────────────────

/// 할당된 메모리 영역을 식별하는 불투명 핸들.
/// 내부 구현을 숨겨 HAL 외부에서 직접 조작 불가.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryHandle(u64);

impl MemoryHandle {
    /// 새 메모리 핸들 생성 (HAL 내부에서만 사용)
    #[doc(hidden)]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// 핸들의 원시 ID 반환
    pub fn raw_id(&self) -> u64 {
        self.0
    }
}

/// 열린 스토리지 스트림 핸들.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageHandle(u64);

impl StorageHandle {
    /// 새 스토리지 핸들 생성 (HAL 내부에서만 사용)
    #[doc(hidden)]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Skill 등록 성공 시 발급되는 capability 토큰.
/// 이 토큰 없이는 어떤 HAL 연산도 요청 불가.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillToken {
    /// 토큰 고유 ID (UUID v4 권장)
    pub token_id: String,
    /// 이 토큰이 허용하는 리소스 목록
    pub allowed_resources: Vec<ResourceType>,
    /// 토큰 만료 시각 (None이면 세션 종료까지)
    pub expires_at: Option<SystemTime>,
}

impl SkillToken {
    /// 주어진 리소스에 대한 접근 권한이 있는지 확인.
    pub fn can_access(&self, resource: &ResourceType) -> bool {
        self.allowed_resources.contains(resource)
    }
}

/// HAL에 등록할 Skill의 메타데이터.
#[derive(Debug, Clone)]
pub struct SkillManifest {
    /// Skill 고유 이름 (예: "music-player", "file-organizer")
    pub name: String,
    /// Skill 버전 (SemVer)
    pub version: String,
    /// 이 Skill이 요청하는 리소스 접근 권한
    pub requested_capabilities: Vec<ResourceType>,
    /// Skill 설명 (사용자에게 표시됨)
    pub description: String,
}

// ─────────────────────────────────────────────
//  섹션 5: Capability 토큰 (접근 제어)
// ─────────────────────────────────────────────

/// HAL 명령 실행 시 반드시 제시해야 하는 접근 권한 토큰.
/// 모든 `AiHalInterface::execute_command()` 호출에 필수.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityToken {
    /// 토큰 소유 Skill의 이름
    pub skill_name: String,
    /// 허용된 리소스 목록
    pub permissions: Vec<ResourceType>,
}

impl CapabilityToken {
    /// 지정 리소스에 대한 권한이 있는지 확인.
    pub fn has_permission(&self, resource: &ResourceType) -> bool {
        self.permissions.contains(resource)
    }
}

// ─────────────────────────────────────────────
//  섹션 6: 감사 로그 (Audit Log)
// ─────────────────────────────────────────────

/// 모든 HAL 연산에 자동 생성되는 감사 엔트리.
/// 보안 감사 및 디버깅에 사용.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// 명령 실행 시각
    pub timestamp: SystemTime,
    /// 요청한 Skill 이름
    pub requestor: String,
    /// 실행된 명령 종류 (Debug 문자열)
    pub command_kind: String,
    /// 성공 여부
    pub succeeded: bool,
}

// ─────────────────────────────────────────────
//  섹션 7: 에러 타입
// ─────────────────────────────────────────────

/// HAL 연산 실패 이유.
///
/// 각 변형은 AI Core가 재시도 여부를 판단하는 데 사용.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalError {
    /// 유효하지 않은 capability 토큰 (권한 없음)
    PermissionDenied {
        /// 요청된 리소스
        resource: ResourceType,
        /// 거부 이유 설명
        reason: String,
    },

    /// 요청한 리소스가 현재 사용 불가 (일시적)
    ResourceUnavailable {
        resource: ResourceType,
    },

    /// 요청한 리소스가 현재 시스템에 존재하지 않음 (영구적)
    ResourceNotFound {
        resource: ResourceType,
    },

    /// 메모리 부족으로 할당 실패
    OutOfMemory {
        /// 요청한 크기 (바이트)
        requested_bytes: usize,
    },

    /// 스토리지 경로 접근 실패
    StoragePathError {
        path: std::path::PathBuf,
        /// OS 에러 메시지
        os_error: String,
    },

    /// 유효하지 않은 명령 파라미터
    InvalidParameter {
        /// 잘못된 파라미터 이름
        param_name: String,
        /// 에러 설명
        message: String,
    },

    /// HAL 내부 오류 (버그)
    InternalError(String),
}

impl fmt::Display for HalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied { resource, reason } => {
                write!(f, "권한 거부: {} — {}", resource, reason)
            }
            Self::ResourceUnavailable { resource } => {
                write!(f, "리소스 사용 불가 (일시적): {}", resource)
            }
            Self::ResourceNotFound { resource } => {
                write!(f, "리소스 없음: {}", resource)
            }
            Self::OutOfMemory { requested_bytes } => {
                write!(f, "메모리 부족: {}바이트 요청", requested_bytes)
            }
            Self::StoragePathError { path, os_error } => {
                write!(f, "스토리지 경로 오류 ({:?}): {}", path, os_error)
            }
            Self::InvalidParameter { param_name, message } => {
                write!(f, "잘못된 파라미터 '{}': {}", param_name, message)
            }
            Self::InternalError(msg) => {
                write!(f, "HAL 내부 오류: {}", msg)
            }
        }
    }
}

impl std::error::Error for HalError {}

// ─────────────────────────────────────────────
//  섹션 8: 핵심 HAL 트레이트 (인터페이스 명세)
// ─────────────────────────────────────────────

/// AI-OS Hardware Abstraction Layer 핵심 인터페이스.
///
/// ## 설계 원칙
/// 1. **모든 연산은 `CapabilityToken`을 요구** — 토큰 없이는 실행 불가
/// 2. **모든 연산은 `AuditEntry`를 생성** — HAL 연산의 완전한 감사 추적
/// 3. **구현체는 플랫폼별로 분리** — Linux, QEMU, Mock 각각 구현
///
/// ## 구현 예시
/// ```rust
/// struct MockHal;
///
/// impl AiHalInterface for MockHal {
///     fn execute_command(
///         &self,
///         token: &CapabilityToken,
///         command: HalCommand,
///     ) -> HalResult {
///         // 테스트용 mock 구현
///         todo!()
///     }
///
///     fn query_state(
///         &self,
///         token: &CapabilityToken,
///         resource: ResourceType,
///     ) -> HalResult {
///         todo!()
///     }
///
///     fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError> {
///         todo!()
///     }
/// }
/// ```
pub trait AiHalInterface: Send + Sync {
    /// HAL 명령을 실행하고 결과와 감사 엔트리를 반환.
    ///
    /// # 인자
    /// - `token`: 실행 권한을 증명하는 capability 토큰
    /// - `command`: 실행할 HAL 명령
    ///
    /// # 반환
    /// - `HalResult`: 실행 결과 + 감사 엔트리 (항상 반환, 실패해도 감사는 기록됨)
    fn execute_command(&self, token: &CapabilityToken, command: HalCommand) -> HalResult;

    /// 리소스 현재 상태를 조회 (read-only, 부수효과 없음).
    ///
    /// `execute_command(HalCommand::QueryState)` 의 편의 래퍼.
    fn query_state(&self, token: &CapabilityToken, resource: ResourceType) -> HalResult;

    /// Skill을 HAL 런타임에 등록하고 capability 토큰 발급.
    ///
    /// Skill은 실행 전에 반드시 등록되어야 함.
    /// 토큰에는 `manifest.requested_capabilities` 기반 권한이 포함됨.
    fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError>;

    /// 현재 HAL 구현의 이름을 반환 (디버깅용).
    fn hal_name(&self) -> &str;

    /// 이 HAL 구현이 지원하는 리소스 목록 반환.
    fn supported_resources(&self) -> Vec<ResourceType>;
}

// ─────────────────────────────────────────────
//  섹션 9: Mock HAL 구현 (테스트용)
// ─────────────────────────────────────────────

/// 테스트 및 개발 환경용 Mock HAL 구현.
///
/// 실제 하드웨어 없이 AI Core 로직을 테스트하기 위해 사용.
/// 프로덕션 코드에서는 절대 사용 금지.
#[cfg(any(test, feature = "mock"))]
pub struct MockHal {
    /// Mock이 반환할 메모리 총량 (기본값: 16GB)
    pub mock_total_memory: u64,
}

#[cfg(any(test, feature = "mock"))]
impl Default for MockHal {
    fn default() -> Self {
        Self {
            mock_total_memory: 16 * 1024 * 1024 * 1024, // 16GB
        }
    }
}

#[cfg(any(test, feature = "mock"))]
impl AiHalInterface for MockHal {
    fn execute_command(&self, token: &CapabilityToken, command: HalCommand) -> HalResult {
        // 명령 종류를 감사 로그용으로 기록
        let command_kind = format!("{:?}", std::mem::discriminant(&command));

        // 권한 확인 (Mock에서도 권한 체크 수행)
        let outcome = match &command {
            HalCommand::QueryState { resource, .. } => {
                if !token.has_permission(resource) {
                    Err(HalError::PermissionDenied {
                        resource: resource.clone(),
                        reason: "Mock 권한 없음".to_string(),
                    })
                } else {
                    self.mock_query_state(resource)
                }
            }
            HalCommand::AllocateMemory { size_bytes, .. } => {
                if !token.has_permission(&ResourceType::Memory) {
                    Err(HalError::PermissionDenied {
                        resource: ResourceType::Memory,
                        reason: "메모리 권한 없음".to_string(),
                    })
                } else if *size_bytes > self.mock_total_memory as usize {
                    Err(HalError::OutOfMemory {
                        requested_bytes: *size_bytes,
                    })
                } else {
                    // Mock: 항상 핸들 ID 42 반환
                    Ok(HalResponse::MemoryAllocated(MemoryHandle::new(42)))
                }
            }
            _ => Ok(HalResponse::Ok),
        };

        HalResult {
            audit: AuditEntry {
                timestamp: SystemTime::now(),
                requestor: token.skill_name.clone(),
                command_kind,
                succeeded: outcome.is_ok(),
            },
            outcome,
        }
    }

    fn query_state(&self, token: &CapabilityToken, resource: ResourceType) -> HalResult {
        self.execute_command(
            token,
            HalCommand::QueryState {
                resource,
                detailed: false,
            },
        )
    }

    fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError> {
        // Mock: 요청한 모든 권한을 그대로 부여 (프로덕션에서는 불가)
        Ok(SkillToken {
            token_id: format!("mock-token-{}", manifest.name),
            allowed_resources: manifest.requested_capabilities,
            expires_at: None,
        })
    }

    fn hal_name(&self) -> &str {
        "MockHal (테스트 전용)"
    }

    fn supported_resources(&self) -> Vec<ResourceType> {
        vec![ResourceType::Memory, ResourceType::Cpu, ResourceType::Storage]
    }
}

#[cfg(any(test, feature = "mock"))]
impl MockHal {
    /// Mock 상태 조회 내부 구현.
    fn mock_query_state(&self, resource: &ResourceType) -> Result<HalResponse, HalError> {
        match resource {
            ResourceType::Memory => Ok(HalResponse::ResourceState(ResourceState::Memory(
                MemoryState {
                    total_bytes: self.mock_total_memory,
                    used_bytes: self.mock_total_memory / 2, // 50% 사용 중으로 가정
                    free_bytes: self.mock_total_memory / 2,
                    page_size: 4096,
                },
            ))),
            ResourceType::Cpu => Ok(HalResponse::ResourceState(ResourceState::Cpu(CpuState {
                logical_cores: 8,
                per_core_usage: vec![0.1, 0.2, 0.05, 0.15, 0.3, 0.1, 0.0, 0.25],
                frequency_mhz: 3_600,
            }))),
            ResourceType::Storage => {
                Ok(HalResponse::ResourceState(ResourceState::Storage(StorageState {
                    total_bytes: 512 * 1024 * 1024 * 1024, // 512GB
                    used_bytes: 200 * 1024 * 1024 * 1024,  // 200GB
                    mount_point: std::path::PathBuf::from("/"),
                })))
            }
            _ => Err(HalError::ResourceNotFound {
                resource: resource.clone(),
            }),
        }
    }
}

// ─────────────────────────────────────────────
//  섹션 10: 단위 테스트
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트용 CapabilityToken 생성 헬퍼.
    fn make_token(permissions: Vec<ResourceType>) -> CapabilityToken {
        CapabilityToken {
            skill_name: "test-skill".to_string(),
            permissions,
        }
    }

    // ── MockHal 기본 동작 테스트 ──

    #[test]
    fn test_mock_hal_name() {
        // MockHal이 올바른 이름을 반환하는지 확인
        let hal = MockHal::default();
        assert!(hal.hal_name().contains("Mock"));
    }

    #[test]
    fn test_register_skill_returns_token() {
        // Skill 등록 후 토큰이 정상 발급되는지 확인
        let hal = MockHal::default();
        let manifest = SkillManifest {
            name: "music-player".to_string(),
            version: "0.1.0".to_string(),
            requested_capabilities: vec![ResourceType::Audio, ResourceType::Storage],
            description: "음악 재생 Skill".to_string(),
        };

        let result = hal.register_skill(manifest);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.token_id, "mock-token-music-player");
        assert!(token.can_access(&ResourceType::Audio));
        assert!(token.can_access(&ResourceType::Storage));
        assert!(!token.can_access(&ResourceType::Cpu)); // 요청하지 않은 권한
    }

    // ── 메모리 쿼리 테스트 ──

    #[test]
    fn test_query_memory_state_with_permission() {
        // 메모리 권한이 있을 때 조회 성공하는지 확인
        let hal = MockHal::default();
        let token = make_token(vec![ResourceType::Memory]);

        let result = hal.query_state(&token, ResourceType::Memory);
        assert!(result.outcome.is_ok());

        if let Ok(HalResponse::ResourceState(ResourceState::Memory(mem))) = result.outcome {
            assert_eq!(mem.total_bytes, 16 * 1024 * 1024 * 1024);
            assert!(mem.usage_ratio() > 0.0 && mem.usage_ratio() <= 1.0);
        } else {
            panic!("예상과 다른 응답 타입");
        }
    }

    #[test]
    fn test_query_memory_state_without_permission() {
        // 메모리 권한 없이 조회 시 PermissionDenied 반환 확인
        let hal = MockHal::default();
        let token = make_token(vec![ResourceType::Cpu]); // 메모리 권한 없음

        let result = hal.query_state(&token, ResourceType::Memory);
        assert!(result.outcome.is_err());

        // 감사 로그에도 실패가 기록되어야 함
        assert!(!result.audit.succeeded);

        if let Err(HalError::PermissionDenied { resource, .. }) = result.outcome {
            assert_eq!(resource, ResourceType::Memory);
        } else {
            panic!("PermissionDenied 에러가 아님");
        }
    }

    // ── 메모리 할당 테스트 ──

    #[test]
    fn test_allocate_memory_success() {
        // 메모리 할당이 정상 동작하는지 확인
        let hal = MockHal::default();
        let token = make_token(vec![ResourceType::Memory]);

        let result = hal.execute_command(
            &token,
            HalCommand::AllocateMemory {
                size_bytes: 1024 * 1024, // 1MB
                alignment: 4096,
                shared: false,
            },
        );

        assert!(result.outcome.is_ok());
        assert!(result.audit.succeeded);
        if let Ok(HalResponse::MemoryAllocated(handle)) = result.outcome {
            assert_eq!(handle.raw_id(), 42); // Mock은 항상 42 반환
        }
    }

    #[test]
    fn test_allocate_memory_out_of_memory() {
        // 시스템 메모리를 초과하는 할당 요청 시 OOM 에러 확인
        let hal = MockHal {
            mock_total_memory: 1024, // 1KB만 있는 Mock
        };
        let token = make_token(vec![ResourceType::Memory]);

        let result = hal.execute_command(
            &token,
            HalCommand::AllocateMemory {
                size_bytes: 1024 * 1024 * 1024, // 1GB 요청 (불가능)
                alignment: 4096,
                shared: false,
            },
        );

        assert!(matches!(
            result.outcome,
            Err(HalError::OutOfMemory { .. })
        ));
    }

    // ── CPU 상태 조회 테스트 ──

    #[test]
    fn test_query_cpu_state() {
        // CPU 상태 조회가 정상 동작하는지 확인
        let hal = MockHal::default();
        let token = make_token(vec![ResourceType::Cpu]);

        let result = hal.query_state(&token, ResourceType::Cpu);
        assert!(result.outcome.is_ok());

        if let Ok(HalResponse::ResourceState(ResourceState::Cpu(cpu))) = result.outcome {
            assert_eq!(cpu.logical_cores, 8);
            assert_eq!(cpu.per_core_usage.len(), 8);
            // 모든 코어 사용률은 0.0 ~ 1.0 범위여야 함
            for usage in &cpu.per_core_usage {
                assert!(*usage >= 0.0 && *usage <= 1.0);
            }
        } else {
            panic!("CPU 상태 응답 타입 불일치");
        }
    }

    // ── 감사 로그 테스트 ──

    #[test]
    fn test_audit_entry_is_always_created() {
        // 성공/실패 모두 감사 엔트리가 생성되는지 확인
        let hal = MockHal::default();

        // 성공 케이스
        let token_ok = make_token(vec![ResourceType::Memory]);
        let result_ok = hal.query_state(&token_ok, ResourceType::Memory);
        assert_eq!(result_ok.audit.requestor, "test-skill");
        assert!(result_ok.audit.succeeded);

        // 실패 케이스
        let token_fail = make_token(vec![]);
        let result_fail = hal.query_state(&token_fail, ResourceType::Memory);
        assert_eq!(result_fail.audit.requestor, "test-skill");
        assert!(!result_fail.audit.succeeded);
    }

    // ── 에러 메시지 출력 테스트 ──

    #[test]
    fn test_hal_error_display() {
        // 에러 메시지가 한국어로 출력되는지 확인
        let err = HalError::OutOfMemory {
            requested_bytes: 1024,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("메모리 부족"));
        assert!(msg.contains("1024"));
    }

    // ── CapabilityToken 권한 확인 테스트 ──

    #[test]
    fn test_capability_token_permission_check() {
        // 토큰 권한 확인 로직 테스트
        let token = CapabilityToken {
            skill_name: "test".to_string(),
            permissions: vec![ResourceType::Memory, ResourceType::Storage],
        };

        assert!(token.has_permission(&ResourceType::Memory));
        assert!(token.has_permission(&ResourceType::Storage));
        assert!(!token.has_permission(&ResourceType::Cpu));
        assert!(!token.has_permission(&ResourceType::Network));
    }

    // ── 지원 리소스 목록 테스트 ──

    #[test]
    fn test_supported_resources() {
        // MockHal이 M0 지원 리소스를 올바르게 반환하는지 확인
        let hal = MockHal::default();
        let resources = hal.supported_resources();

        assert!(resources.contains(&ResourceType::Memory));
        assert!(resources.contains(&ResourceType::Cpu));
        assert!(resources.contains(&ResourceType::Storage));
    }
}
