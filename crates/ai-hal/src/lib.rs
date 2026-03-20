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
//! AI-OS Hardware Abstraction Layer (HAL) 루트 크레이트.
//!
//! ## 모듈 구조
//! ```text
//! ai-hal
//! ├── lib.rs      — 공통 타입 + AiHalInterface trait + LinuxHal + MockHal
//! ├── error.rs    — HalError 에러 타입 (SyscallFailed 포함)
//! ├── memory.rs   — LinuxMemoryHal (/proc/meminfo + mmap/munmap)
//! ├── cpu.rs      — LinuxCpuHal (/proc/stat + sched_setaffinity)
//! └── storage.rs  — LinuxStorageHal (statfs + 블록 I/O)
//! ```
//!
//! ## 아키텍처 위치
//! ```text
//! AI Core (Python) → ai-core-bridge → ai-hal → Linux Kernel → Hardware
//! ```
//!
//! ## 빠른 시작
//! ```rust
//! use ai_hal::{LinuxHal, AiHalInterface, ResourceType, CapabilityToken};
//!
//! let hal = LinuxHal::new();
//! let token = CapabilityToken {
//!     skill_name: "my-skill".to_string(),
//!     permissions: vec![ResourceType::Memory],
//! };
//! let result = hal.query_state(&token, ResourceType::Memory);
//! assert!(result.outcome.is_ok());
//! ```

#![warn(missing_docs, clippy::all)]

// ─────────────────────────────────────────────
//  서브모듈 선언
// ─────────────────────────────────────────────

/// HAL 에러 타입 (SyscallFailed 포함)
pub mod error;
/// Linux 메모리 직접 제어 (/proc/meminfo + mmap)
pub mod memory;
/// Linux CPU 스케줄링 제어 (/proc/stat + sched_setaffinity)
pub mod cpu;
/// Linux 블록 스토리지 제어 (statfs + 직접 I/O)
pub mod storage;
/// STRIDE 기반 HAL 보안 방어 레이어 (파라미터 인젝션 / 토큰 위조 / DoS)
pub mod security;

// ─────────────────────────────────────────────
//  주요 타입 재내보내기 (public API)
// ─────────────────────────────────────────────

pub use error::HalError;
pub use memory::LinuxMemoryHal;
pub use cpu::LinuxCpuHal;
pub use storage::LinuxStorageHal;

use std::fmt;
use std::time::SystemTime;

// ─────────────────────────────────────────────
//  섹션 1: 리소스 타입
// ─────────────────────────────────────────────

/// HAL이 직접 제어할 수 있는 하드웨어 리소스 종류.
///
/// M0: Memory / Cpu / Storage 구현
/// M2+: Gpu / Audio / Camera 추가 예정
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResourceType {
    /// 시스템 메인 메모리 (RAM)
    Memory,
    /// CPU 코어 및 스케줄링
    Cpu,
    /// 영구 저장장치 (HDD/SSD/NVMe)
    Storage,
    /// 그래픽 처리 장치 (M2+)
    Gpu,
    /// 오디오 입출력 (M2+)
    Audio,
    /// 카메라 입력 (M2+)
    Camera,
    /// 네트워크 인터페이스
    Network,
    /// 디스플레이 (M2+)
    Display,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Memory  => "Memory",
            Self::Cpu     => "CPU",
            Self::Storage => "Storage",
            Self::Gpu     => "GPU",
            Self::Audio   => "Audio",
            Self::Camera  => "Camera",
            Self::Network => "Network",
            Self::Display => "Display",
        };
        write!(f, "{}", s)
    }
}

// ─────────────────────────────────────────────
//  섹션 2: HAL 명령 타입
// ─────────────────────────────────────────────

/// AI Core → HAL 명령 열거형.
///
/// 모든 명령은 `CapabilityToken`을 동반해야 실행됨.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HalCommand {
    /// 리소스 현재 상태 조회 (read-only)
    QueryState {
        /// 조회 대상 리소스
        resource: ResourceType,
        /// true: 상세 정보 포함, false: 요약만
        detailed: bool,
    },
    /// 메모리 영역 할당 요청
    AllocateMemory {
        /// 요청 바이트 수
        size_bytes: usize,
        /// 메모리 정렬 요구 (2의 거듭제곱, 최소 4096)
        alignment: usize,
        /// 다른 프로세스와 공유 가능 여부 (MAP_SHARED vs MAP_PRIVATE)
        shared: bool,
    },
    /// 할당된 메모리 해제
    FreeMemory {
        /// 해제할 메모리 핸들
        handle: MemoryHandle,
    },
    /// CPU 스케줄링 힌트 (강제 아님, OS 권고)
    CpuSchedulingHint {
        /// 대상 태스크 PID (0 = 호출 프로세스)
        pid: u32,
        /// 우선순위 (0 = 최저, 255 = 최고)
        priority: u8,
        /// 선호 CPU 코어 번호 (None = OS 결정)
        preferred_core: Option<usize>,
    },
    /// 스토리지 경로 읽기 스트림 열기
    OpenStorageRead {
        /// 읽을 파일 경로
        path: std::path::PathBuf,
    },
    /// 스토리지 경로 쓰기 스트림 열기
    OpenStorageWrite {
        /// 쓸 파일 경로
        path: std::path::PathBuf,
        /// 파일 없으면 생성 여부
        create_if_missing: bool,
    },
    /// Skill을 HAL 런타임에 등록
    RegisterSkill {
        /// Skill 메타데이터
        manifest: SkillManifest,
    },
}

// ─────────────────────────────────────────────
//  섹션 3: 응답 및 상태 타입
// ─────────────────────────────────────────────

/// HAL 명령 실행 결과 묶음 (감사 엔트리 + 결과).
///
/// 실패해도 `audit`은 항상 기록됨 — 보안 감사 추적 보장.
#[derive(Debug)]
pub struct HalResult {
    /// 감사 엔트리: 항상 생성됨
    pub audit: AuditEntry,
    /// 실행 결과: Ok(HalResponse) 또는 Err(HalError)
    pub outcome: Result<HalResponse, HalError>,
}

/// HAL 명령 성공 응답 종류.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HalResponse {
    /// 리소스 상태 응답
    ResourceState(ResourceState),
    /// 메모리 할당 성공
    MemoryAllocated(MemoryHandle),
    /// 스토리지 스트림 핸들
    StorageHandle(StorageHandle),
    /// Skill 등록 성공 토큰
    SkillRegistered(SkillToken),
    /// 응답 데이터 없는 성공 (FreeMemory 등)
    Ok,
}

/// 리소스 상태 스냅샷.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ResourceState {
    /// 메모리 상태 (/proc/meminfo 기반)
    Memory(MemoryState),
    /// CPU 상태 (/proc/stat 기반)
    Cpu(CpuState),
    /// 스토리지 상태 (statfs 기반)
    Storage(StorageState),
}

/// 메모리 상태 스냅샷.
///
/// `/proc/meminfo` 파싱 결과.
#[derive(Debug, Clone)]
pub struct MemoryState {
    /// 총 물리 메모리 (바이트)
    pub total_bytes: u64,
    /// 사용 중인 메모리 (바이트) = total - available
    pub used_bytes: u64,
    /// 커널이 사용 가능하다고 보고하는 메모리 (MemAvailable)
    pub available_bytes: u64,
    /// 버퍼 캐시 (바이트)
    pub buffers_bytes: u64,
    /// 페이지 캐시 (바이트)
    pub cached_bytes: u64,
    /// 시스템 페이지 크기 (바이트, 보통 4096)
    pub page_size: usize,
}

impl MemoryState {
    /// 메모리 사용률 (0.0 ~ 1.0).
    ///
    /// 계산식: (total - available) / total
    pub fn usage_ratio(&self) -> f64 {
        if self.total_bytes == 0 { return 0.0; }
        self.used_bytes as f64 / self.total_bytes as f64
    }

    /// 사용 중인 메모리를 사람이 읽기 편한 형식으로 반환 (GiB).
    pub fn used_gib(&self) -> f64 {
        self.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// CPU 상태 스냅샷.
///
/// `/proc/stat` 파싱 결과.
#[derive(Debug, Clone)]
pub struct CpuState {
    /// 논리 코어 수
    pub logical_cores: usize,
    /// 각 코어별 사용률 (0.0 ~ 1.0).
    /// 두 번의 `/proc/stat` 읽기 델타로 계산.
    pub per_core_usage: Vec<f64>,
    /// 전체 CPU 평균 사용률
    pub total_usage: f64,
    /// 현재 CPU 주파수 MHz
    /// (`/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq`)
    pub frequency_mhz: u64,
    /// CPU 모델 이름 (`/proc/cpuinfo`)
    pub model_name: String,
}

/// 스토리지 상태 스냅샷.
///
/// `statfs()` syscall 결과.
#[derive(Debug, Clone)]
pub struct StorageState {
    /// 총 용량 (바이트)
    pub total_bytes: u64,
    /// 사용 중인 용량 (바이트)
    pub used_bytes: u64,
    /// 사용 가능한 용량 (바이트, root 제외)
    pub available_bytes: u64,
    /// 파일시스템 기본 블록 크기 (바이트, 보통 4096)
    pub block_size: u32,
    /// 조회한 마운트 포인트 경로
    pub mount_point: std::path::PathBuf,
    /// 파일시스템 타입 (예: "ext4", "btrfs", "tmpfs")
    pub fs_type: String,
}

impl StorageState {
    /// 스토리지 사용률 (0.0 ~ 1.0).
    pub fn usage_ratio(&self) -> f64 {
        if self.total_bytes == 0 { return 0.0; }
        self.used_bytes as f64 / self.total_bytes as f64
    }
}

// ─────────────────────────────────────────────
//  섹션 4: 핸들 및 토큰 타입
// ─────────────────────────────────────────────

/// mmap으로 할당된 메모리 영역의 불투명 핸들.
///
/// HAL 외부에서 직접 포인터 접근 불가 — 안전성 보장.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryHandle(u64);

impl MemoryHandle {
    /// 새 핸들 생성 (HAL 내부 전용).
    #[doc(hidden)]
    pub fn new(id: u64) -> Self { Self(id) }
    /// 핸들 원시 ID 반환.
    pub fn raw_id(&self) -> u64 { self.0 }
}

/// 열린 스토리지 스트림 핸들.
///
/// `Fd` 변형: Linux 저수준 파일 디스크립터 기반 핸들.
/// `Id` 변형: 추상 핸들 ID (Mock/테스트용).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageHandle {
    /// Linux 파일 디스크립터 (open(2) 반환값)
    Fd(i32),
    /// 불투명 핸들 ID (테스트/추상 계층용)
    Id(u64),
}

impl StorageHandle {
    /// 추상 ID 핸들 생성 (HAL 내부 전용).
    #[doc(hidden)]
    pub fn new(id: u64) -> Self { Self::Id(id) }
    /// 핸들 원시 ID 반환.
    pub fn raw_id(&self) -> u64 {
        match self {
            Self::Fd(fd) => *fd as u64,
            Self::Id(id) => *id,
        }
    }
}

/// Skill 등록 후 발급되는 capability 토큰.
///
/// 이 토큰 없이는 어떤 HAL 연산도 실행 불가.
/// 토큰은 등록된 `requested_capabilities` 기반으로 생성됨.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillToken {
    /// 전역 고유 토큰 ID (UUID v4 권장)
    pub token_id: String,
    /// 허용된 리소스 목록
    pub allowed_resources: Vec<ResourceType>,
    /// 만료 시각 (None = 세션 종료까지)
    pub expires_at: Option<SystemTime>,
}

impl SkillToken {
    /// 특정 리소스에 대한 접근 권한이 있는지 확인.
    pub fn can_access(&self, resource: &ResourceType) -> bool {
        self.allowed_resources.contains(resource)
    }

    /// 토큰이 만료됐는지 확인.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < SystemTime::now())
            .unwrap_or(false)
    }
}

/// Skill 등록 메타데이터.
#[derive(Debug, Clone)]
pub struct SkillManifest {
    /// Skill 고유 이름 (예: "music-player", "file-organizer")
    pub name: String,
    /// SemVer 버전 문자열 (예: "0.1.0")
    pub version: String,
    /// 요청하는 리소스 접근 권한 목록
    pub requested_capabilities: Vec<ResourceType>,
    /// 사용자에게 표시할 설명
    pub description: String,
}

// ─────────────────────────────────────────────
//  섹션 5: Capability 토큰 (접근 제어)
// ─────────────────────────────────────────────

/// HAL 명령 실행 시 필수로 제시해야 하는 접근 제어 토큰.
///
/// capability-based security 모델 구현.
/// 참조: Saltzer & Schroeder (1975), "The Protection of Information in Computer Systems"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityToken {
    /// 토큰 소유 Skill 이름
    pub skill_name: String,
    /// 허용된 리소스 목록
    pub permissions: Vec<ResourceType>,
}

impl CapabilityToken {
    /// 새 CapabilityToken 생성.
    pub fn new(permissions: Vec<ResourceType>, skill_name: &str) -> Self {
        Self { skill_name: skill_name.to_string(), permissions }
    }

    /// 특정 리소스에 대한 권한이 있는지 확인.
    pub fn has_permission(&self, resource: &ResourceType) -> bool {
        self.permissions.contains(resource)
    }

    /// `has_permission`의 간결한 별칭 (integration test / AI Core 친화적).
    pub fn allows(&self, resource: &ResourceType) -> bool {
        self.has_permission(resource)
    }
}

// ─────────────────────────────────────────────
//  섹션 6: 감사 로그 타입
// ─────────────────────────────────────────────

/// 모든 HAL 연산에 자동 생성되는 감사 엔트리.
///
/// 성공/실패 여부와 무관하게 항상 기록됨.
/// SECURITY: 감사 로그는 절대 삭제 불가해야 함 (append-only).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// 연산 실행 시각
    pub timestamp: SystemTime,
    /// 요청 Skill 이름
    pub requestor: String,
    /// 실행된 명령 종류 (enum 변형 이름)
    pub command_kind: String,
    /// 성공 여부
    pub succeeded: bool,
    /// 실패 이유 (실패한 경우만)
    pub failure_reason: Option<String>,
}

impl AuditEntry {
    /// 성공 감사 엔트리 생성 헬퍼.
    pub fn success(requestor: &str, command_kind: &str) -> Self {
        Self {
            timestamp: SystemTime::now(),
            requestor: requestor.to_string(),
            command_kind: command_kind.to_string(),
            succeeded: true,
            failure_reason: None,
        }
    }

    /// 실패 감사 엔트리 생성 헬퍼.
    pub fn failure(requestor: &str, command_kind: &str, reason: &str) -> Self {
        Self {
            timestamp: SystemTime::now(),
            requestor: requestor.to_string(),
            command_kind: command_kind.to_string(),
            succeeded: false,
            failure_reason: Some(reason.to_string()),
        }
    }
}

// ─────────────────────────────────────────────
//  섹션 7: 핵심 HAL 트레이트
// ─────────────────────────────────────────────

/// AI-OS Hardware Abstraction Layer 핵심 인터페이스 트레이트.
///
/// ## 불변 조건 (Invariants)
/// 1. **모든 연산은 `CapabilityToken`을 요구** — 토큰 없이 실행 불가
/// 2. **모든 연산은 `AuditEntry`를 생성** — 보안 감사 추적 보장
/// 3. **Send + Sync** — 멀티스레드 Skill 런타임에서 공유 가능
///
/// ## 구현 목록
/// - `LinuxHal` (M0): 실제 Linux 커널 브리지
/// - `MockHal` (테스트): 하드웨어 없이 동작하는 테스트용 구현
///
/// ## 구현 예시
/// ```rust,ignore
/// struct MyHal;
/// impl AiHalInterface for MyHal {
///     fn execute_command(&self, token: &CapabilityToken, cmd: HalCommand) -> HalResult {
///         // 구현
///         todo!()
///     }
///     fn query_state(&self, token: &CapabilityToken, resource: ResourceType) -> HalResult {
///         self.execute_command(token, HalCommand::QueryState { resource, detailed: false })
///     }
///     fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError> {
///         // 구현
///         todo!()
///     }
///     fn hal_name(&self) -> &str { "MyHal" }
///     fn supported_resources(&self) -> Vec<ResourceType> { vec![] }
/// }
/// ```
pub trait AiHalInterface: Send + Sync {
    /// HAL 명령을 실행하고 결과와 감사 엔트리를 반환.
    ///
    /// # 인자
    /// - `token`: capability 토큰 (권한 증명)
    /// - `command`: 실행할 HAL 명령
    ///
    /// # 반환
    /// `HalResult`: 항상 반환됨 (실패해도 audit은 기록됨)
    fn execute_command(&self, token: &CapabilityToken, command: HalCommand) -> HalResult;

    /// 리소스 상태를 조회 (read-only, 부수효과 없음).
    ///
    /// `execute_command(HalCommand::QueryState { ... })` 의 편의 래퍼.
    fn query_state(&self, token: &CapabilityToken, resource: ResourceType) -> HalResult;

    /// Skill을 HAL에 등록하고 capability 토큰 발급.
    ///
    /// 등록 없이는 HAL 명령 실행 불가.
    fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError>;

    /// HAL 구현 이름 (디버깅/로깅용).
    fn hal_name(&self) -> &str;

    /// 이 HAL 구현이 지원하는 리소스 목록.
    fn supported_resources(&self) -> Vec<ResourceType>;
}

// ─────────────────────────────────────────────
//  섹션 8: LinuxHal — 실제 Linux 구현체 (M0)
// ─────────────────────────────────────────────

/// Linux 커널을 직접 제어하는 HAL 구현체.
///
/// 내부적으로 `LinuxMemoryHal`, `LinuxCpuHal`, `LinuxStorageHal`을 조합.
///
/// ## 지원 syscall 브리지
/// - 메모리: `/proc/meminfo` 읽기, `mmap`/`munmap`
/// - CPU: `/proc/stat` 읽기, `sched_setaffinity`
/// - 스토리지: `statfs`, `open`/`read`/`write`/`close`
pub struct LinuxHal {
    /// 메모리 HAL 서브시스템
    pub memory: LinuxMemoryHal,
    /// CPU HAL 서브시스템
    pub cpu: LinuxCpuHal,
    /// 스토리지 HAL 서브시스템
    pub storage: LinuxStorageHal,
}

impl LinuxHal {
    /// 새 LinuxHal 인스턴스 생성.
    pub fn new() -> Self {
        Self {
            memory: LinuxMemoryHal::new(),
            cpu: LinuxCpuHal::new(),
            storage: LinuxStorageHal::new(),
        }
    }
}

impl Default for LinuxHal {
    fn default() -> Self { Self::new() }
}

impl AiHalInterface for LinuxHal {
    fn execute_command(&self, token: &CapabilityToken, command: HalCommand) -> HalResult {
        let cmd_kind = format!("{:?}", std::mem::discriminant(&command));

        // 권한 확인 선행 (pre-flight check)
        let resource = command_to_resource(&command);
        if let Some(res) = &resource {
            if !token.has_permission(res) {
                return HalResult {
                    audit: AuditEntry::failure(
                        &token.skill_name,
                        &cmd_kind,
                        &format!("권한 없음: {}", res),
                    ),
                    outcome: Err(HalError::PermissionDenied {
                        resource: res.clone(),
                        reason: format!(
                            "Skill '{}' 은 {} 리소스 접근 권한이 없습니다",
                            token.skill_name, res
                        ),
                    }),
                };
            }
        }

        // 서브시스템에 위임
        let outcome = match command {
            HalCommand::QueryState { resource, .. } => match resource {
                ResourceType::Memory => self.memory.query_state_inner(),
                ResourceType::Cpu => self.cpu.query_state_inner(),
                ResourceType::Storage => self.storage.query_state_inner(),
                _ => Err(HalError::ResourceNotFound { resource }),
            },
            HalCommand::AllocateMemory { size_bytes, alignment, shared } => {
                self.memory
                    .allocate(size_bytes, alignment, shared)
                    .map(HalResponse::MemoryAllocated)
            }
            HalCommand::FreeMemory { handle } => {
                self.memory.free(handle).map(|_| HalResponse::Ok)
            }
            HalCommand::CpuSchedulingHint { pid, preferred_core, .. } => {
                if let Some(core) = preferred_core {
                    self.cpu.set_affinity(pid, core).map(|_| HalResponse::Ok)
                } else {
                    Ok(HalResponse::Ok) // 힌트 무시 (선호 코어 없음)
                }
            }
            HalCommand::OpenStorageRead { path } => {
                self.storage.open_read(&path).map(HalResponse::StorageHandle)
            }
            HalCommand::OpenStorageWrite { path, create_if_missing } => {
                self.storage
                    .open_write(&path, create_if_missing)
                    .map(HalResponse::StorageHandle)
            }
            HalCommand::RegisterSkill { manifest } => {
                self.register_skill(manifest).map(HalResponse::SkillRegistered)
            }
        };

        HalResult {
            audit: if outcome.is_ok() {
                AuditEntry::success(&token.skill_name, &cmd_kind)
            } else {
                AuditEntry::failure(
                    &token.skill_name,
                    &cmd_kind,
                    &outcome.as_ref().unwrap_err().to_string(),
                )
            },
            outcome,
        }
    }

    fn query_state(&self, token: &CapabilityToken, resource: ResourceType) -> HalResult {
        self.execute_command(
            token,
            HalCommand::QueryState { resource, detailed: false },
        )
    }

    fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError> {
        // M0: 요청 권한을 그대로 부여 (M1에서 심사 로직 추가 예정)
        Ok(SkillToken {
            token_id: format!(
                "linux-{}-{}-{}",
                manifest.name,
                manifest.version,
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
            allowed_resources: manifest.requested_capabilities,
            expires_at: None,
        })
    }

    fn hal_name(&self) -> &str { "LinuxHal" }

    fn supported_resources(&self) -> Vec<ResourceType> {
        vec![ResourceType::Memory, ResourceType::Cpu, ResourceType::Storage]
    }
}

/// HalCommand에서 주 리소스 타입을 추출하는 헬퍼.
fn command_to_resource(cmd: &HalCommand) -> Option<ResourceType> {
    match cmd {
        HalCommand::QueryState { resource, .. } => Some(resource.clone()),
        HalCommand::AllocateMemory { .. } | HalCommand::FreeMemory { .. } => {
            Some(ResourceType::Memory)
        }
        HalCommand::CpuSchedulingHint { .. } => Some(ResourceType::Cpu),
        HalCommand::OpenStorageRead { .. } | HalCommand::OpenStorageWrite { .. } => {
            Some(ResourceType::Storage)
        }
        HalCommand::RegisterSkill { .. } => None,
    }
}

// ─────────────────────────────────────────────
//  섹션 9: MockHal — 테스트 전용 구현
// ─────────────────────────────────────────────

/// 하드웨어 없이 동작하는 테스트 전용 Mock HAL.
///
/// `#[cfg(any(test, feature = "mock"))]` 조건부 컴파일.
/// 프로덕션 코드에서 절대 사용 금지.
#[cfg(any(test, feature = "mock"))]
pub struct MockHal {
    /// Mock 시스템 총 메모리 (바이트, 기본 16GB)
    pub mock_total_memory: u64,
    /// Mock CPU 코어 수 (기본 8)
    pub mock_cpu_cores: usize,
    /// Mock 스토리지 총 용량 (바이트, 기본 512GB)
    pub mock_total_storage: u64,
}

#[cfg(any(test, feature = "mock"))]
impl Default for MockHal {
    fn default() -> Self {
        Self {
            mock_total_memory:  16 * 1024 * 1024 * 1024, // 16 GiB
            mock_cpu_cores: 8,
            mock_total_storage: 512 * 1024 * 1024 * 1024, // 512 GiB
        }
    }
}

#[cfg(any(test, feature = "mock"))]
impl AiHalInterface for MockHal {
    fn execute_command(&self, token: &CapabilityToken, command: HalCommand) -> HalResult {
        let cmd_kind = format!("{:?}", std::mem::discriminant(&command));

        let resource = command_to_resource(&command);
        if let Some(res) = &resource {
            if !token.has_permission(res) {
                return HalResult {
                    audit: AuditEntry::failure(&token.skill_name, &cmd_kind, "권한 없음"),
                    outcome: Err(HalError::PermissionDenied {
                        resource: res.clone(),
                        reason: "Mock 권한 없음".to_string(),
                    }),
                };
            }
        }

        let outcome: Result<HalResponse, HalError> = match command {
            HalCommand::QueryState { resource, .. } => self.mock_query(&resource),
            HalCommand::AllocateMemory { size_bytes, .. } => {
                if size_bytes as u64 > self.mock_total_memory {
                    Err(HalError::OutOfMemory {
                        requested_bytes: size_bytes,
                        available_bytes: Some(self.mock_total_memory as usize / 2),
                    })
                } else {
                    Ok(HalResponse::MemoryAllocated(MemoryHandle::new(0xDEAD_BEEF)))
                }
            }
            HalCommand::FreeMemory { .. }
            | HalCommand::CpuSchedulingHint { .. }
            | HalCommand::OpenStorageRead { .. }
            | HalCommand::OpenStorageWrite { .. } => Ok(HalResponse::Ok),
            HalCommand::RegisterSkill { manifest } => {
                self.register_skill(manifest).map(HalResponse::SkillRegistered)
            }
        };

        HalResult {
            audit: if outcome.is_ok() {
                AuditEntry::success(&token.skill_name, &cmd_kind)
            } else {
                AuditEntry::failure(
                    &token.skill_name,
                    &cmd_kind,
                    &outcome.as_ref().unwrap_err().to_string(),
                )
            },
            outcome,
        }
    }

    fn query_state(&self, token: &CapabilityToken, resource: ResourceType) -> HalResult {
        self.execute_command(token, HalCommand::QueryState { resource, detailed: false })
    }

    fn register_skill(&self, manifest: SkillManifest) -> Result<SkillToken, HalError> {
        Ok(SkillToken {
            token_id: format!("mock-{}", manifest.name),
            allowed_resources: manifest.requested_capabilities,
            expires_at: None,
        })
    }

    fn hal_name(&self) -> &str { "MockHal (테스트 전용)" }

    fn supported_resources(&self) -> Vec<ResourceType> {
        vec![ResourceType::Memory, ResourceType::Cpu, ResourceType::Storage]
    }
}

#[cfg(any(test, feature = "mock"))]
impl MockHal {
    /// 새 MockHal 인스턴스 생성 (기본값 사용).
    pub fn new() -> Self { Self::default() }

    /// Mock 리소스 상태 조회 내부 구현.
    fn mock_query(&self, resource: &ResourceType) -> Result<HalResponse, HalError> {
        match resource {
            ResourceType::Memory => Ok(HalResponse::ResourceState(ResourceState::Memory(
                MemoryState {
                    total_bytes:     self.mock_total_memory,
                    used_bytes:      self.mock_total_memory / 2,
                    available_bytes: self.mock_total_memory / 2,
                    buffers_bytes:   256 * 1024 * 1024,
                    cached_bytes:    1024 * 1024 * 1024,
                    page_size: 4096,
                },
            ))),
            ResourceType::Cpu => Ok(HalResponse::ResourceState(ResourceState::Cpu(CpuState {
                logical_cores: self.mock_cpu_cores,
                per_core_usage: vec![0.10, 0.20, 0.05, 0.15, 0.30, 0.10, 0.00, 0.25]
                    .into_iter()
                    .take(self.mock_cpu_cores)
                    .collect(),
                total_usage: 0.144,
                frequency_mhz: 3_600,
                model_name: "Mock CPU (테스트용)".to_string(),
            }))),
            ResourceType::Storage => Ok(HalResponse::ResourceState(ResourceState::Storage(
                StorageState {
                    total_bytes:     self.mock_total_storage,
                    used_bytes:      self.mock_total_storage * 2 / 5,
                    available_bytes: self.mock_total_storage * 3 / 5,
                    block_size:      4096,
                    mount_point: std::path::PathBuf::from("/"),
                    fs_type: "mock_fs".to_string(),
                },
            ))),
            _ => Err(HalError::ResourceNotFound { resource: resource.clone() }),
        }
    }
}

// ─────────────────────────────────────────────
//  단위 테스트 (lib.rs 레벨)
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_token(perms: Vec<ResourceType>) -> CapabilityToken {
        CapabilityToken { skill_name: "test-skill".to_string(), permissions: perms }
    }

    #[test]
    fn test_mock_register_and_query_memory() {
        // 등록 → 토큰 발급 → 메모리 조회 전체 흐름 검증
        let hal = MockHal::default();
        let manifest = SkillManifest {
            name: "test-skill".to_string(),
            version: "0.1.0".to_string(),
            requested_capabilities: vec![ResourceType::Memory],
            description: "테스트".to_string(),
        };
        let token_result = hal.register_skill(manifest);
        assert!(token_result.is_ok());

        let skill_token = token_result.unwrap();
        let cap_token = CapabilityToken {
            skill_name: "test-skill".to_string(),
            permissions: skill_token.allowed_resources.clone(),
        };

        let result = hal.query_state(&cap_token, ResourceType::Memory);
        assert!(result.outcome.is_ok());
        assert!(result.audit.succeeded);
    }

    #[test]
    fn test_mock_permission_denied_returns_audit() {
        // 권한 없을 때도 audit 엔트리가 생성되는지 확인
        let hal = MockHal::default();
        let token = make_token(vec![]); // 권한 없음
        let result = hal.query_state(&token, ResourceType::Memory);

        assert!(result.outcome.is_err());
        assert!(!result.audit.succeeded);
        assert!(result.audit.failure_reason.is_some());
    }

    #[test]
    fn test_mock_oom_with_available_bytes() {
        // OutOfMemory 에러에 available_bytes가 포함되는지 확인
        let hal = MockHal {
            mock_total_memory: 1024, // 1KiB만 있는 환경
            ..Default::default()
        };
        let token = make_token(vec![ResourceType::Memory]);
        let result = hal.execute_command(
            &token,
            HalCommand::AllocateMemory { size_bytes: 1024 * 1024, alignment: 4096, shared: false },
        );
        assert!(matches!(
            result.outcome,
            Err(HalError::OutOfMemory { available_bytes: Some(_), .. })
        ));
    }

    #[test]
    fn test_skill_token_expiry() {
        // 만료되지 않은 토큰 확인
        let token = SkillToken {
            token_id: "test".to_string(),
            allowed_resources: vec![ResourceType::Memory],
            expires_at: None,
        };
        assert!(!token.is_expired());
        assert!(token.can_access(&ResourceType::Memory));
        assert!(!token.can_access(&ResourceType::Cpu));
    }

    #[test]
    fn test_audit_entry_helpers() {
        // AuditEntry 헬퍼 메서드 검증
        let ok = AuditEntry::success("skill-a", "QueryState");
        assert!(ok.succeeded);
        assert!(ok.failure_reason.is_none());

        let fail = AuditEntry::failure("skill-b", "AllocateMemory", "OOM");
        assert!(!fail.succeeded);
        assert_eq!(fail.failure_reason.as_deref(), Some("OOM"));
    }

    #[test]
    fn test_memory_state_usage_ratio() {
        let mem = MemoryState {
            total_bytes: 16_000,
            used_bytes: 8_000,
            available_bytes: 8_000,
            buffers_bytes: 0,
            cached_bytes: 0,
            page_size: 4096,
        };
        let ratio = mem.usage_ratio();
        assert!((ratio - 0.5).abs() < 1e-9, "예상 0.5, 실제 {}", ratio);
    }

    #[test]
    fn test_storage_state_usage_ratio() {
        let s = StorageState {
            total_bytes: 100,
            used_bytes: 40,
            available_bytes: 60,
            block_size: 4096,
            mount_point: std::path::PathBuf::from("/"),
            fs_type: "ext4".to_string(),
        };
        let ratio = s.usage_ratio();
        assert!((ratio - 0.4).abs() < 1e-9);
    }

    #[test]
    fn test_linux_hal_name() {
        // LinuxHal이 올바른 이름 반환하는지 확인
        let hal = LinuxHal::new();
        assert_eq!(hal.hal_name(), "LinuxHal");
    }

    #[test]
    fn test_mock_hal_name() {
        let hal = MockHal::default();
        assert!(hal.hal_name().contains("Mock"));
    }
}
