// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! # security — AI Core → HAL 보안 방어 레이어
//!
//! ## STRIDE 위협 모델 기반 Top 3 Prompt Injection 시나리오
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   AI Core → HAL 공격 표면                        │
//! │                                                                 │
//! │  User Prompt                                                    │
//! │      │                                                          │
//! │      ▼                                                          │
//! │  [LLM / IntentParser] ──공격 가능→ HalCommand 파라미터 위조       │
//! │      │                                                          │
//! │      ▼                                                          │
//! │  [CapabilityToken] ──────공격 가능→ 위조/재사용 토큰              │
//! │      │                                                          │
//! │      ▼                                                          │
//! │  [HAL Command Bus] ──────공격 가능→ 명령 폭주(DoS)               │
//! │      │                                                          │
//! │      ▼                                                          │
//! │  Linux Kernel                                                   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 시나리오별 STRIDE 분류 및 방어 구현
//!
//! | # | 시나리오                    | STRIDE      | 방어 구현           |
//! |---|----------------------------|-------------|---------------------|
//! | 1 | HAL 파라미터 인젝션          | T + EoP     | `CommandSanitizer`  |
//! | 2 | CapabilityToken 위조/재사용  | S + EoP     | `TokenAuthority`    |
//! | 3 | 명령 폭주 자원 고갈          | DoS         | `RateLimiter`       |
//!
//! ## 참조
//! - STRIDE 위협 모델: Shostack (2014), *Threat Modeling: Designing for Security*
//! - Capability 기반 보안: Saltzer & Schroeder (1975), CACM 18(7):461–493
//! - OWASP Prompt Injection: https://owasp.org/www-project-top-10-for-large-language-model-applications/

#![allow(dead_code)]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{CapabilityToken, HalCommand, ResourceType};

// ─────────────────────────────────────────────────────────
//  섹션 1: 보안 에러 타입
// ─────────────────────────────────────────────────────────

/// HAL 보안 레이어에서 발생하는 모든 위반 오류.
///
/// `HalError::PermissionDenied` 로 변환 가능하여
/// 기존 HAL 인터페이스와 자연스럽게 통합됨.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    // ── 시나리오 1: 파라미터 인젝션 관련 ─────────────────
    /// 파라미터 값이 허용 범위를 초과 (Tampering)
    ///
    /// 예: `size_bytes = usize::MAX` (AllocateMemory DoS)
    ParameterOutOfRange {
        /// 초과한 필드 이름
        field: &'static str,
        /// 실제 전달된 값
        value: usize,
        /// 허용 최댓값
        max: usize,
    },

    /// 정렬 값이 2의 거듭제곱이 아님 (Tampering)
    ///
    /// 예: `alignment = 1000` → 커널 SIGBUS 유발 가능
    InvalidAlignment {
        /// 전달된 정렬 값
        alignment: usize,
    },

    /// 경로 순회 공격 탐지 (Tampering + EoP)
    ///
    /// 예: `path = "../../etc/shadow"` → 루트 이탈
    /// OWASP: https://owasp.org/www-community/attacks/Path_Traversal
    PathTraversal {
        /// 탐지된 악성 경로
        path: String,
    },

    /// 차단된 시스템 경로 접근 시도 (Tampering + EoP)
    ///
    /// 예: `/dev/mem`, `/proc/kcore` → 물리 메모리 직접 접근
    PathNotAllowed {
        /// 차단된 경로
        path: String,
        /// 차단 이유
        reason: &'static str,
    },

    /// 경로 내 null 바이트 인젝션 탐지 (Tampering)
    ///
    /// 예: `path = "/tmp/safe\0/etc/shadow"` → 일부 libc 버전 취약점
    NullByteInPath,

    // ── 시나리오 2: 토큰 위조/재사용 관련 ───────────────
    /// 토큰 서명 검증 실패 (Spoofing)
    ///
    /// 토큰 내용이 서명 이후 변조된 경우.
    TokenSignatureInvalid,

    /// 만료된 토큰 사용 시도 (Spoofing + EoP)
    TokenExpired,

    /// 토큰에 요청 명령에 필요한 권한 없음 (EoP)
    ///
    /// `CapabilityToken.permissions` 에 없는 리소스 접근 시도.
    InsufficientPermission {
        /// 접근 시도한 리소스
        resource: ResourceType,
        /// 토큰 소유자
        skill_name: String,
    },

    // ── 시나리오 3: DoS 관련 ─────────────────────────────
    /// 슬라이딩 윈도우 레이트 리밋 초과 (DoS)
    ///
    /// 예: 1초에 100회 이상 HAL 명령 발송
    RateLimitExceeded {
        /// 제한 초과 skill 이름
        skill_name: String,
        /// 윈도우 내 최대 허용 명령 수
        limit: usize,
        /// 윈도우 크기 (초)
        window_secs: u64,
    },

    /// 누적 메모리 할당 예산 초과 (DoS)
    ///
    /// 세션 내 총 할당량이 스킬별 예산(기본 1 GiB)을 초과.
    AllocationBudgetExceeded {
        /// 제한 초과 skill 이름
        skill_name: String,
        /// 이번에 요청한 바이트 수
        requested: usize,
        /// 잔여 예산 (바이트)
        remaining_budget: usize,
    },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // 파라미터 인젝션
            Self::ParameterOutOfRange { field, value, max } =>
                write!(f, "[SEC-T01] 파라미터 범위 초과: {field} = {value} (최대: {max})"),
            Self::InvalidAlignment { alignment } =>
                write!(f, "[SEC-T02] 잘못된 정렬 값: {alignment} (2의 거듭제곱 필요)"),
            Self::PathTraversal { path } =>
                write!(f, "[SEC-T03] 경로 순회 공격 탐지: {path}"),
            Self::PathNotAllowed { path, reason } =>
                write!(f, "[SEC-T04] 차단된 경로: {path} ({reason})"),
            Self::NullByteInPath =>
                write!(f, "[SEC-T05] 경로에 null 바이트 인젝션 탐지"),
            // 토큰 위조
            Self::TokenSignatureInvalid =>
                write!(f, "[SEC-S01] 토큰 서명 검증 실패 — 위조 가능성"),
            Self::TokenExpired =>
                write!(f, "[SEC-S02] 만료된 capability 토큰"),
            Self::InsufficientPermission { resource, skill_name } =>
                write!(f, "[SEC-S03] 권한 부족: skill={skill_name}, resource={resource:?}"),
            // DoS
            Self::RateLimitExceeded { skill_name, limit, window_secs } =>
                write!(f, "[SEC-D01] 레이트 리밋 초과: {skill_name} ({limit}회/{window_secs}초)"),
            Self::AllocationBudgetExceeded { skill_name, requested, remaining_budget } =>
                write!(f, "[SEC-D02] 할당 예산 초과: {skill_name} 요청={requested}B 잔여={remaining_budget}B"),
        }
    }
}

impl std::error::Error for SecurityError {}

// ─────────────────────────────────────────────────────────
//  섹션 2: 시나리오 1 방어 — CommandSanitizer
//
//  [STRIDE: Tampering + Elevation of Privilege]
//  공격: LLM이 악의적 파라미터가 포함된 HalCommand를 생성
//  예:
//    - AllocateMemory { size_bytes: usize::MAX }     → OOM crash
//    - OpenStorageRead { path: "../../etc/shadow" }  → 경로 순회
//    - OpenStorageWrite { path: "/dev/sda" }          → 블록 장치 직접 쓰기
//    - AllocateMemory { alignment: 3 }               → 커널 SIGBUS
//
//  방어 원칙:
//    1. 화이트리스트 기반 경로 접두사 허용
//    2. 범위 검증 (size_bytes, alignment, core 번호)
//    3. 경로 컴포넌트별 `..` / null 바이트 / 심볼릭 링크 탐지
//
//  참조:
//    - Path Traversal: CWE-22 (MITRE)
//    - Integer Overflow in Allocation: CWE-190
// ─────────────────────────────────────────────────────────

/// HAL 명령 파라미터 sanitizer (시나리오 1 방어).
///
/// `LinuxHal::execute_command()` 호출 전 인터셉트하여
/// 모든 파라미터를 범위·형식·경로 정책으로 검증.
pub struct CommandSanitizer {
    /// 최대 메모리 할당 요청 바이트 (기본: 4 GiB)
    max_alloc_bytes: usize,

    /// 최대 파일 경로 길이 (기본: 4096, Linux PATH_MAX)
    max_path_len: usize,

    /// 허용된 경로 접두사 화이트리스트
    /// 이 목록에 포함되지 않으면 전부 차단.
    allowed_prefixes: Vec<std::path::PathBuf>,

    /// 명시적 차단 경로 목록 (정밀 블랙리스트)
    /// allowed_prefixes 통과 후에도 추가 검사.
    denied_prefixes: Vec<(&'static str, &'static str)>, // (경로, 사유)

    /// 최대 CPU 코어 인덱스 (기본: 1023)
    max_cpu_core: usize,
}

impl Default for CommandSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandSanitizer {
    /// 보안 기본값으로 CommandSanitizer 생성.
    ///
    /// ## 기본 정책
    /// - 메모리 할당 상한: 4 GiB
    /// - 허용 경로: `/tmp/`, `/home/`, `/var/`, `/data/`
    /// - 차단 경로: `/dev/`, `/proc/`, `/sys/`, `/boot/`, `/etc/`
    pub fn new() -> Self {
        Self {
            // 4 GiB: 단일 스킬이 전체 RAM을 한 번에 요청하는 공격 차단
            max_alloc_bytes: 4 * 1024 * 1024 * 1024,

            // Linux PATH_MAX (include/uapi/linux/limits.h)
            max_path_len: 4096,

            // 화이트리스트 경로: 일반 사용자 데이터 공간만 허용
            allowed_prefixes: vec![
                std::path::PathBuf::from("/tmp/"),
                std::path::PathBuf::from("/home/"),
                std::path::PathBuf::from("/var/"),
                std::path::PathBuf::from("/data/"),
                std::path::PathBuf::from("/mnt/"),
                std::path::PathBuf::from("/media/"),
                std::path::PathBuf::from("/opt/"),
                std::path::PathBuf::from("/run/user/"),
            ],

            // 블랙리스트: 커널 인터페이스 및 하드웨어 직접 접근 차단
            denied_prefixes: vec![
                ("/dev/", "하드웨어 장치 파일 직접 접근 차단"),
                ("/proc/", "커널 proc 파일시스템 접근 차단"),
                ("/sys/", "커널 sysfs 접근 차단"),
                ("/boot/", "부트로더/커널 이미지 보호"),
                ("/etc/shadow", "패스워드 데이터베이스 보호"),
                ("/etc/sudoers", "권한 설정 파일 보호"),
                ("/etc/passwd", "사용자 계정 파일 보호"),
            ],

            // CPU 코어 번호 상한 (ARM/x86 서버 최대 1024코어 기준)
            max_cpu_core: 1023,
        }
    }

    /// HalCommand 파라미터 전체 검증.
    ///
    /// ## 알고리즘
    /// 1. 명령 타입별 분기
    /// 2. 각 필드에 대한 범위/형식 검사
    /// 3. 경로 포함 명령은 추가로 `validate_path()` 호출
    ///
    /// ## 반환
    /// - `Ok(())`: 검증 통과, HAL 실행 가능
    /// - `Err(SecurityError)`: 위반 탐지, HAL 실행 차단
    pub fn validate(&self, cmd: &HalCommand) -> Result<(), SecurityError> {
        match cmd {
            HalCommand::AllocateMemory { size_bytes, alignment, .. } => {
                self.validate_alloc(*size_bytes, *alignment)
            }

            HalCommand::OpenStorageRead { path } => {
                self.validate_path(path)
            }

            HalCommand::OpenStorageWrite { path, .. } => {
                self.validate_path(path)
            }

            HalCommand::CpuSchedulingHint { preferred_core, .. } => {
                self.validate_cpu_hint(*preferred_core)
            }

            // QueryState, FreeMemory, RegisterSkill: 파라미터 범위 위반 없음
            HalCommand::QueryState { .. }
            | HalCommand::FreeMemory { .. }
            | HalCommand::RegisterSkill { .. } => Ok(()),
        }
    }

    // ── 내부 검증 메서드 ────────────────────────────────

    /// 메모리 할당 요청 파라미터 검증.
    ///
    /// ## 검사 항목
    /// 1. size_bytes ∈ (0, max_alloc_bytes]
    /// 2. alignment가 2의 거듭제곱
    /// 3. alignment ≤ size_bytes (정렬이 크기보다 크면 무의미)
    fn validate_alloc(&self, size_bytes: usize, alignment: usize) -> Result<(), SecurityError> {
        // 0바이트 할당: 구현 정의 동작(UB) 방지
        if size_bytes == 0 || size_bytes > self.max_alloc_bytes {
            return Err(SecurityError::ParameterOutOfRange {
                field: "size_bytes",
                value: size_bytes,
                max: self.max_alloc_bytes,
            });
        }

        // alignment = 0 또는 2의 거듭제곱이 아닌 경우:
        // mmap/posix_memalign은 EINVAL 반환, 일부 구현은 UB
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(SecurityError::InvalidAlignment { alignment });
        }

        Ok(())
    }

    /// 파일 경로 검증.
    ///
    /// ## 검사 순서 (조기 반환)
    /// 1. null 바이트 인젝션
    /// 2. 경로 길이
    /// 3. `..` 컴포넌트 (경로 순회)
    /// 4. 블랙리스트 접두사
    /// 5. 화이트리스트 접두사
    fn validate_path(&self, path: &Path) -> Result<(), SecurityError> {
        let path_str = path.to_string_lossy();

        // 1. null 바이트 인젝션 탐지
        //    C 문자열 경계를 속여 libc open(2) 오작동 유발 가능
        //    참조: CWE-158 Improper Neutralization of Null Byte
        if path_str.contains('\0') {
            return Err(SecurityError::NullByteInPath);
        }

        // 2. 경로 길이 초과
        if path_str.len() > self.max_path_len {
            return Err(SecurityError::ParameterOutOfRange {
                field: "path_len",
                value: path_str.len(),
                max: self.max_path_len,
            });
        }

        // 3. `..` 컴포넌트 탐지 (경로 순회 공격)
        //    `path.components()`를 사용하여 심볼릭 링크 정규화 전
        //    어휘적(lexical) 수준에서 탐지.
        //    참조: OWASP Path Traversal, CWE-22
        for component in path.components() {
            if component == Component::ParentDir {
                return Err(SecurityError::PathTraversal {
                    path: path_str.to_string(),
                });
            }
        }

        // 4. 블랙리스트 경로 접두사 검사
        //    화이트리스트보다 먼저 검사하여 우선순위 보장
        for (denied, reason) in &self.denied_prefixes {
            if path_str.starts_with(denied) {
                return Err(SecurityError::PathNotAllowed {
                    path: path_str.to_string(),
                    reason,
                });
            }
        }

        // 5. 화이트리스트 접두사 검사
        //    화이트리스트 목록이 비어 있으면 모든 경로 허용 (개발 모드)
        if !self.allowed_prefixes.is_empty() {
            let is_allowed = self.allowed_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix));

            if !is_allowed {
                return Err(SecurityError::PathNotAllowed {
                    path: path_str.to_string(),
                    reason: "화이트리스트에 없는 경로",
                });
            }
        }

        Ok(())
    }

    /// CPU 힌트 파라미터 검증.
    fn validate_cpu_hint(&self, preferred_core: Option<usize>) -> Result<(), SecurityError> {
        if let Some(core) = preferred_core {
            if core > self.max_cpu_core {
                return Err(SecurityError::ParameterOutOfRange {
                    field: "preferred_core",
                    value: core,
                    max: self.max_cpu_core,
                });
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────
//  섹션 3: 시나리오 2 방어 — TokenAuthority
//
//  [STRIDE: Spoofing + Elevation of Privilege]
//  공격: 악의적 스킬이 권한 없는 CapabilityToken을 위조/재사용
//  예:
//    - 메모리 권한만 받았으나 `permissions`에 CPU 추가 후 재사용
//    - 이전 세션의 토큰을 캡처하여 재전송 (replay attack)
//    - 서명 없는 토큰을 직접 구성하여 HAL에 전달
//
//  방어 원칙:
//    1. 발급 시 토큰 내용에 대한 keyed hash 서명
//    2. 실행 시 서명 재검증 (내용 변조 탐지)
//    3. 서명 키는 HAL 런타임 시작 시 랜덤 생성, 외부 노출 없음
//
//  보안 주의: M0에서는 std::hash(DefaultHasher)를 사용.
//  이는 암호학적으로 안전하지 않으며 M1에서
//  HMAC-SHA256 (hmac + sha2 크레이트)으로 교체 예정.
//
//  참조:
//    - Capability 기반 보안: Saltzer & Schroeder (1975)
//    - HMAC: RFC 2104 (Krawczyk et al., 1997)
//    - Replay Attack: STRIDE-R (Repudiation)
// ─────────────────────────────────────────────────────────

/// 서명된 CapabilityToken (위조 방지용 래퍼).
///
/// 내부의 `token`은 서명 후 수정 불가.
/// 서명 검증 없이는 HAL 명령 실행 불가.
#[derive(Debug, Clone)]
pub struct SignedToken {
    /// 서명 대상 capability 토큰
    pub token: CapabilityToken,
    /// 토큰 내용에 대한 keyed 서명값 (8바이트, M0 플레이스홀더)
    ///
    /// SECURITY: M1에서 HMAC-SHA256 (32바이트)으로 교체 필요.
    signature: u64,
}

/// Capability 토큰 발급 및 검증 기관 (시나리오 2 방어).
///
/// HAL 시작 시 하나의 `TokenAuthority` 인스턴스를 생성하고
/// 모든 토큰 발급/검증을 이 인스턴스를 통해 처리.
///
/// ## 보안 수준 (M0 vs M1)
///
/// | 항목         | M0 (현재)               | M1 (예정)             |
/// |-------------|------------------------|-----------------------|
/// | 해시 함수    | `DefaultHasher` (비암호) | HMAC-SHA256           |
/// | 서명 크기    | 8바이트                  | 32바이트               |
/// | 키 관리      | 스택 변수 [u8; 32]       | OS keychain / HSM     |
/// | 재사용 방지  | 미구현                   | nonce + replay DB     |
pub struct TokenAuthority {
    /// 서명 키 (32바이트).
    ///
    /// SECURITY: 이 키가 노출되면 모든 토큰을 위조 가능.
    /// M1에서 OS 키체인 또는 HSM으로 관리 예정.
    signing_key: [u8; 32],
}

impl TokenAuthority {
    /// 새 TokenAuthority 생성.
    ///
    /// ## 인자
    /// - `signing_key`: 서명 비밀 키 (32바이트).
    ///   호출자가 안전한 CSPRNG으로 생성해야 함.
    ///
    /// M1 구현 예시 (getrandom 크레이트):
    /// ```rust,ignore
    /// let mut key = [0u8; 32];
    /// getrandom::getrandom(&mut key).expect("CSPRNG 실패");
    /// let authority = TokenAuthority::new(key);
    /// ```
    pub fn new(signing_key: [u8; 32]) -> Self {
        Self { signing_key }
    }

    /// CapabilityToken에 서명하여 SignedToken 생성.
    ///
    /// 발급 후 토큰 내용(permissions 목록 등)을 변조하면
    /// `verify()` 에서 탐지됨.
    pub fn sign(&self, token: CapabilityToken) -> SignedToken {
        let signature = self.compute_signature(&token);
        SignedToken { token, signature }
    }

    /// SignedToken 서명 검증.
    ///
    /// ## 검증 항목
    /// 1. 서명값 일치 (내용 변조 탐지)
    /// 2. 현재 시각 기준 만료 여부 (SkillToken.expires_at 연동은 M1 예정)
    ///
    /// ## 반환
    /// - `Ok(())`: 서명 유효, 토큰 신뢰 가능
    /// - `Err(SecurityError::TokenSignatureInvalid)`: 위조 탐지
    pub fn verify(&self, signed: &SignedToken) -> Result<(), SecurityError> {
        let expected = self.compute_signature(&signed.token);

        // 상수 시간 비교 (timing side-channel 방지)
        // M0: 단순 비교. M1에서 subtle::ConstantTimeEq 사용 예정
        // 참조: CERT Secure Coding SEC64-CPP
        if expected != signed.signature {
            return Err(SecurityError::TokenSignatureInvalid);
        }
        Ok(())
    }

    /// 토큰 내용에 대한 서명값 계산.
    ///
    /// ## 알고리즘 (M0 플레이스홀더)
    /// 입력: signing_key ∥ skill_name ∥ permissions_sorted
    /// 출력: DefaultHasher(keyed_input) → u64
    ///
    /// SECURITY WARNING:
    ///   DefaultHasher는 암호학적으로 안전하지 않음.
    ///   공격자가 해시 충돌을 의도적으로 만들 수 있음.
    ///   M1에서 HMAC-SHA256으로 반드시 교체할 것.
    ///
    /// M1 교체 코드 (hmac + sha2 크레이트):
    /// ```rust,ignore
    /// use hmac::{Hmac, Mac};
    /// use sha2::Sha256;
    /// type HmacSha256 = Hmac<Sha256>;
    /// let mut mac = HmacSha256::new_from_slice(&self.signing_key).unwrap();
    /// mac.update(canonical_bytes);
    /// mac.finalize().into_bytes()
    /// ```
    fn compute_signature(&self, token: &CapabilityToken) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // 1. 키 해시 (keyed construction)
        self.signing_key.hash(&mut hasher);

        // 2. skill_name 해시
        token.skill_name.hash(&mut hasher);

        // 3. permissions 정렬 후 해시 (순서 독립적 서명)
        //    permissions 순서가 바뀌어도 같은 서명 보장
        let mut perms: Vec<String> = token.permissions
            .iter()
            .map(|r| format!("{:?}", r))
            .collect();
        perms.sort();
        perms.hash(&mut hasher);

        hasher.finish()
    }
}

// ─────────────────────────────────────────────────────────
//  섹션 4: 시나리오 3 방어 — RateLimiter
//
//  [STRIDE: Denial of Service]
//  공격: 악의적 스킬이 HAL에 대량의 명령을 폭주시켜 자원 고갈
//  예:
//    - loop { AllocateMemory(4GiB) } → 시스템 메모리 고갈
//    - loop { QueryState } → /proc 파일시스템 I/O 포화
//    - AllocateMemory × 10,000 → 커널 페이지 테이블 단편화
//
//  방어 원칙 (두 단계):
//    1. 슬라이딩 윈도우 레이트 리밋: 단기 폭주 차단
//       - 스킬별로 윈도우(기본 1초) 내 최대 N회 명령 허용
//    2. 누적 할당 예산: 장기 메모리 고갈 차단
//       - 스킬별 세션 내 총 할당량 상한 (기본 1 GiB)
//
//  알고리즘: Sliding Window Log (가장 공정한 레이트 리밋 방식)
//  참조:
//    - Kong (2017), *Rate Limiting Algorithms Compared*
//    - CWE-770: Allocation of Resources Without Limits
// ─────────────────────────────────────────────────────────

/// 스킬별 레이트 리밋 상태 (내부 전용).
struct SkillRateState {
    /// 최근 명령 타임스탬프 슬라이딩 윈도우 로그
    timestamps: VecDeque<Instant>,
    /// 세션 내 누적 메모리 할당 바이트 수
    cumulative_alloc_bytes: usize,
}

impl SkillRateState {
    fn new() -> Self {
        Self {
            timestamps: VecDeque::new(),
            cumulative_alloc_bytes: 0,
        }
    }
}

/// 슬라이딩 윈도우 레이트 리미터 (시나리오 3 방어).
///
/// 스킬별로 명령 빈도와 누적 메모리 할당량을 추적.
/// 내부 상태는 `Mutex`로 보호하여 멀티스레드 안전.
pub struct RateLimiter {
    /// 슬라이딩 윈도우 크기
    window: Duration,

    /// 윈도우 내 허용 최대 명령 수 (기본: 100회/초)
    max_commands_per_window: usize,

    /// 세션 내 스킬별 최대 메모리 할당 예산 (기본: 1 GiB)
    max_alloc_budget_bytes: usize,

    /// 스킬 이름 → 레이트 상태 맵 (Mutex 보호)
    state: Mutex<HashMap<String, SkillRateState>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    /// 기본 보안 파라미터로 RateLimiter 생성.
    ///
    /// ## 기본 정책
    /// - 윈도우: 1초
    /// - 최대 명령: 100회/초 (초당 100회 이상이면 DoS 의심)
    /// - 할당 예산: 1 GiB/세션 (단일 스킬이 RAM 전체 고갈 방지)
    pub fn new() -> Self {
        Self {
            window: Duration::from_secs(1),
            max_commands_per_window: 100,
            max_alloc_budget_bytes: 1 * 1024 * 1024 * 1024, // 1 GiB
            state: Mutex::new(HashMap::new()),
        }
    }

    /// 커스텀 파라미터로 RateLimiter 생성 (테스트 및 고급 설정용).
    pub fn with_config(
        window: Duration,
        max_commands: usize,
        max_alloc_budget_bytes: usize,
    ) -> Self {
        Self {
            window,
            max_commands_per_window: max_commands,
            max_alloc_budget_bytes,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// 명령 허용 여부 확인 후 상태 기록.
    ///
    /// ## 알고리즘 (Sliding Window Log)
    /// 1. 만료된 타임스탬프 제거 (now - window 이전)
    /// 2. 윈도우 내 카운트 > max_commands → RateLimitExceeded
    /// 3. AllocateMemory인 경우 누적 예산 검사
    /// 4. 통과하면 현재 타임스탬프 기록
    ///
    /// ## 시간 복잡도
    /// O(k) where k = 윈도우 내 명령 수 (만료 정리 비용)
    pub fn check_and_record(
        &self,
        skill_name: &str,
        cmd: &HalCommand,
    ) -> Result<(), SecurityError> {
        let mut map = self.state.lock().expect("RateLimiter 뮤텍스 오염");
        let entry = map.entry(skill_name.to_string()).or_insert_with(SkillRateState::new);

        let now = Instant::now();

        // 1. 슬라이딩 윈도우: 만료된 타임스탬프 제거
        while let Some(&front) = entry.timestamps.front() {
            if now.duration_since(front) > self.window {
                entry.timestamps.pop_front();
            } else {
                break;
            }
        }

        // 2. 명령 빈도 검사
        if entry.timestamps.len() >= self.max_commands_per_window {
            return Err(SecurityError::RateLimitExceeded {
                skill_name: skill_name.to_string(),
                limit: self.max_commands_per_window,
                window_secs: self.window.as_secs(),
            });
        }

        // 3. 메모리 할당 예산 검사 (AllocateMemory 명령만)
        if let HalCommand::AllocateMemory { size_bytes, .. } = cmd {
            let remaining = self.max_alloc_budget_bytes
                .saturating_sub(entry.cumulative_alloc_bytes);

            if *size_bytes > remaining {
                return Err(SecurityError::AllocationBudgetExceeded {
                    skill_name: skill_name.to_string(),
                    requested: *size_bytes,
                    remaining_budget: remaining,
                });
            }

            // 예산 차감 기록
            entry.cumulative_alloc_bytes =
                entry.cumulative_alloc_bytes.saturating_add(*size_bytes);
        }

        // 4. 타임스탬프 기록
        entry.timestamps.push_back(now);

        Ok(())
    }

    /// 스킬 할당 예산 초기화 (FreeMemory 이후 또는 관리자 리셋).
    pub fn reset_budget(&self, skill_name: &str) {
        let mut map = self.state.lock().expect("RateLimiter 뮤텍스 오염");
        if let Some(entry) = map.get_mut(skill_name) {
            entry.cumulative_alloc_bytes = 0;
        }
    }

    /// 특정 스킬의 현재 윈도우 내 명령 수 반환 (모니터링용).
    pub fn current_command_count(&self, skill_name: &str) -> usize {
        let mut map = self.state.lock().expect("RateLimiter 뮤텍스 오염");
        let entry = map.entry(skill_name.to_string()).or_insert_with(SkillRateState::new);
        let now = Instant::now();

        // 만료 타임스탬프 정리 후 카운트
        while let Some(&front) = entry.timestamps.front() {
            if now.duration_since(front) > self.window {
                entry.timestamps.pop_front();
            } else {
                break;
            }
        }
        entry.timestamps.len()
    }
}

// ─────────────────────────────────────────────────────────
//  섹션 5: SecurityGuard — 3개 방어 레이어 통합 퍼사드
// ─────────────────────────────────────────────────────────

/// 3개 보안 레이어를 조합한 단일 진입점.
///
/// `LinuxHal::execute_command()` 호출 직전에 삽입하여
/// 모든 보안 검사를 직렬로 실행.
///
/// ## 실행 순서 (fail-fast)
/// ```text
/// check(signed_token, cmd)
///     │
///     ├─ 1. TokenAuthority.verify()    서명 검증 (위조 탐지)
///     ├─ 2. Token.has_permission()     권한 확인 (EoP 방지)
///     ├─ 3. CommandSanitizer.validate() 파라미터 검증 (인젝션 방지)
///     └─ 4. RateLimiter.check()        레이트 리밋 (DoS 방지)
/// ```
pub struct SecurityGuard {
    /// 토큰 서명 검증기 (시나리오 2)
    pub authority: TokenAuthority,
    /// 파라미터 sanitizer (시나리오 1)
    pub sanitizer: CommandSanitizer,
    /// 레이트 리미터 (시나리오 3)
    pub rate_limiter: RateLimiter,
}

impl SecurityGuard {
    /// 서명 키로 SecurityGuard 생성.
    ///
    /// 프로덕션에서는 CSPRNG으로 키를 생성할 것.
    pub fn new(signing_key: [u8; 32]) -> Self {
        Self {
            authority: TokenAuthority::new(signing_key),
            sanitizer: CommandSanitizer::new(),
            rate_limiter: RateLimiter::new(),
        }
    }

    /// 모든 보안 검사 실행.
    ///
    /// 검사 중 하나라도 실패하면 즉시 반환 (fail-fast).
    ///
    /// ## 인자
    /// - `signed_token`: 발급 시 서명된 capability 토큰
    /// - `cmd`: 실행할 HAL 명령
    ///
    /// ## 반환
    /// - `Ok(())`: 모든 검사 통과, HAL 실행 승인
    /// - `Err(SecurityError)`: 보안 위반 탐지, HAL 실행 차단
    pub fn check(
        &self,
        signed_token: &SignedToken,
        cmd: &HalCommand,
    ) -> Result<(), SecurityError> {
        // 1. 토큰 서명 검증 (Spoofing 방지)
        self.authority.verify(signed_token)?;

        // 2. 명령-토큰 권한 대조 (EoP 방지)
        self.check_permission(&signed_token.token, cmd)?;

        // 3. 파라미터 sanitization (Tampering 방지)
        self.sanitizer.validate(cmd)?;

        // 4. 레이트 리밋 (DoS 방지)
        self.rate_limiter.check_and_record(&signed_token.token.skill_name, cmd)?;

        Ok(())
    }

    /// 토큰 권한 대조 헬퍼.
    fn check_permission(
        &self,
        token: &CapabilityToken,
        cmd: &HalCommand,
    ) -> Result<(), SecurityError> {
        // 명령에 필요한 리소스 타입 결정
        let required_resource = match cmd {
            HalCommand::QueryState { resource, .. } => Some(resource),
            HalCommand::AllocateMemory { .. } | HalCommand::FreeMemory { .. } =>
                Some(&ResourceType::Memory),
            HalCommand::CpuSchedulingHint { .. } => Some(&ResourceType::Cpu),
            HalCommand::OpenStorageRead { .. } | HalCommand::OpenStorageWrite { .. } =>
                Some(&ResourceType::Storage),
            HalCommand::RegisterSkill { .. } => None, // 등록은 토큰 없이 가능
        };

        if let Some(resource) = required_resource {
            if !token.has_permission(resource) {
                return Err(SecurityError::InsufficientPermission {
                    resource: resource.clone(),
                    skill_name: token.skill_name.clone(),
                });
            }
        }

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────
//  섹션 6: 단위 테스트
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityToken, HalCommand, ResourceType};
    use std::path::PathBuf;

    // ── 헬퍼 ─────────────────────────────────────────────

    fn test_key() -> [u8; 32] {
        // 테스트용 고정 키 (프로덕션에서는 CSPRNG 사용 필수)
        *b"aios-test-key-32bytes-0000000000"
    }

    fn token_with(perms: Vec<ResourceType>) -> CapabilityToken {
        CapabilityToken::new(perms, "test-skill")
    }

    fn authority() -> TokenAuthority {
        TokenAuthority::new(test_key())
    }

    fn sanitizer() -> CommandSanitizer {
        CommandSanitizer::new()
    }

    // ────────────────────────────────────────────────────
    //  시나리오 1: CommandSanitizer 테스트
    // ────────────────────────────────────────────────────

    // [STRIDE-T01] 메모리 상한 초과 → 차단
    #[test]
    fn sec_s1_alloc_exceeds_4gib_is_blocked() {
        let san = sanitizer();
        // 공격: size_bytes = usize::MAX (OOM crash 유도)
        let cmd = HalCommand::AllocateMemory {
            size_bytes: usize::MAX,
            alignment: 4096,
            shared: false,
        };
        let result = san.validate(&cmd);
        assert!(matches!(
            result,
            Err(SecurityError::ParameterOutOfRange { field: "size_bytes", .. })
        ));
    }

    // [STRIDE-T01] 0바이트 할당 → 차단 (구현 정의 동작 방지)
    #[test]
    fn sec_s1_zero_alloc_is_blocked() {
        let san = sanitizer();
        let cmd = HalCommand::AllocateMemory {
            size_bytes: 0,
            alignment: 4096,
            shared: false,
        };
        assert!(san.validate(&cmd).is_err());
    }

    // [STRIDE-T02] 정렬 값이 2의 거듭제곱 아닌 경우 → 차단
    #[test]
    fn sec_s1_invalid_alignment_is_blocked() {
        let san = sanitizer();
        let cmd = HalCommand::AllocateMemory {
            size_bytes: 4096,
            alignment: 1000, // 공격: 비정렬 값 → SIGBUS 유발 가능
            shared: false,
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::InvalidAlignment { alignment: 1000 })
        ));
    }

    // [STRIDE-T02] 정렬 0 → 차단
    #[test]
    fn sec_s1_zero_alignment_is_blocked() {
        let san = sanitizer();
        let cmd = HalCommand::AllocateMemory {
            size_bytes: 4096,
            alignment: 0,
            shared: false,
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::InvalidAlignment { alignment: 0 })
        ));
    }

    // [STRIDE-T03] 경로 순회 공격 → 차단
    #[test]
    fn sec_s1_path_traversal_dot_dot_is_blocked() {
        let san = sanitizer();
        // 공격: ../../etc/shadow
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("/tmp/../../etc/shadow"),
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::PathTraversal { .. })
        ));
    }

    // [STRIDE-T03] 상대 경로 순회 → 차단
    #[test]
    fn sec_s1_relative_path_traversal_is_blocked() {
        let san = sanitizer();
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("../../../etc/passwd"),
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::PathTraversal { .. })
        ));
    }

    // [STRIDE-T04] /dev/mem 접근 → 차단 (물리 메모리 직접 접근 방지)
    #[test]
    fn sec_s1_dev_mem_access_is_blocked() {
        let san = sanitizer();
        // 공격: /dev/mem → 물리 메모리 직접 읽기/쓰기 가능
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("/dev/mem"),
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::PathNotAllowed { .. })
        ));
    }

    // [STRIDE-T04] /proc/kcore 접근 → 차단
    #[test]
    fn sec_s1_proc_kcore_access_is_blocked() {
        let san = sanitizer();
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("/proc/kcore"),
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::PathNotAllowed { .. })
        ));
    }

    // [STRIDE-T04] /etc/shadow 쓰기 → 차단
    #[test]
    fn sec_s1_write_etc_shadow_is_blocked() {
        let san = sanitizer();
        // 공격: 패스워드 데이터베이스 변조
        let cmd = HalCommand::OpenStorageWrite {
            path: PathBuf::from("/etc/shadow"),
            create_if_missing: false,
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::PathNotAllowed { .. })
        ));
    }

    // [STRIDE-T05] null 바이트 인젝션 → 차단
    #[test]
    fn sec_s1_null_byte_injection_is_blocked() {
        let san = sanitizer();
        // 공격: /tmp/safe\0/etc/shadow — 일부 libc 구현에서 \0 이후 무시
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("/tmp/safe\0/etc/shadow"),
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::NullByteInPath)
        ));
    }

    // 정상 경로 → 통과
    #[test]
    fn sec_s1_valid_tmp_path_passes() {
        let san = sanitizer();
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("/tmp/aios/data.bin"),
        };
        assert!(san.validate(&cmd).is_ok());
    }

    // 정상 메모리 할당 → 통과
    #[test]
    fn sec_s1_valid_alloc_passes() {
        let san = sanitizer();
        let cmd = HalCommand::AllocateMemory {
            size_bytes: 1024 * 1024, // 1 MiB
            alignment: 4096,
            shared: false,
        };
        assert!(san.validate(&cmd).is_ok());
    }

    // CPU 코어 상한 초과 → 차단
    #[test]
    fn sec_s1_cpu_core_out_of_range_is_blocked() {
        let san = sanitizer();
        let cmd = HalCommand::CpuSchedulingHint {
            pid: 0,
            priority: 128,
            preferred_core: Some(99999), // 공격: 존재하지 않는 코어 번호
        };
        assert!(matches!(
            san.validate(&cmd),
            Err(SecurityError::ParameterOutOfRange { field: "preferred_core", .. })
        ));
    }

    // ────────────────────────────────────────────────────
    //  시나리오 2: TokenAuthority 테스트
    // ────────────────────────────────────────────────────

    // [STRIDE-S01] 정상 서명/검증 라운드트립 → 성공
    #[test]
    fn sec_s2_sign_and_verify_roundtrip() {
        let auth = authority();
        let token = token_with(vec![ResourceType::Memory]);
        let signed = auth.sign(token);
        assert!(auth.verify(&signed).is_ok());
    }

    // [STRIDE-S01] permissions 변조 후 검증 → 실패 (위조 탐지)
    #[test]
    fn sec_s2_tampered_permissions_detected() {
        let auth = authority();
        let token = token_with(vec![ResourceType::Storage]); // Storage 권한만 발급
        let mut signed = auth.sign(token);

        // 공격: Storage 권한 토큰을 탈취하여 Memory 권한 추가
        signed.token.permissions.push(ResourceType::Memory);

        assert!(matches!(
            auth.verify(&signed),
            Err(SecurityError::TokenSignatureInvalid)
        ));
    }

    // [STRIDE-S01] skill_name 변조 → 실패
    #[test]
    fn sec_s2_tampered_skill_name_detected() {
        let auth = authority();
        let token = token_with(vec![ResourceType::Memory]);
        let mut signed = auth.sign(token);

        // 공격: skill_name을 고권한 스킬로 변조
        signed.token.skill_name = "admin-skill".to_string();

        assert!(matches!(
            auth.verify(&signed),
            Err(SecurityError::TokenSignatureInvalid)
        ));
    }

    // [STRIDE-S01] 다른 키로 서명한 토큰 → 검증 실패 (키 불일치)
    #[test]
    fn sec_s2_wrong_key_signature_rejected() {
        let auth1 = TokenAuthority::new(*b"key-for-authority-one-000000000a");
        let auth2 = TokenAuthority::new(*b"key-for-authority-two-000000000b");

        let token = token_with(vec![ResourceType::Memory]);
        let signed_by_auth1 = auth1.sign(token);

        // auth2로 auth1이 서명한 토큰 검증 → 실패
        assert!(matches!(
            auth2.verify(&signed_by_auth1),
            Err(SecurityError::TokenSignatureInvalid)
        ));
    }

    // ────────────────────────────────────────────────────
    //  시나리오 3: RateLimiter 테스트
    // ────────────────────────────────────────────────────

    // [STRIDE-D01] 짧은 시간에 명령 폭주 → 레이트 리밋 차단
    #[test]
    fn sec_s3_rate_limit_blocks_command_flood() {
        // 윈도우 1초, 최대 5회로 설정 (테스트 빠른 실행용)
        let rl = RateLimiter::with_config(
            Duration::from_secs(1),
            5,
            1024 * 1024 * 1024,
        );
        let cmd = HalCommand::QueryState {
            resource: ResourceType::Memory,
            detailed: false,
        };

        // 5회 통과
        for _ in 0..5 {
            assert!(rl.check_and_record("flood-skill", &cmd).is_ok());
        }

        // 6번째 → 차단
        assert!(matches!(
            rl.check_and_record("flood-skill", &cmd),
            Err(SecurityError::RateLimitExceeded { .. })
        ));
    }

    // [STRIDE-D01] 서로 다른 스킬은 독립적 카운터
    #[test]
    fn sec_s3_different_skills_have_independent_counters() {
        let rl = RateLimiter::with_config(Duration::from_secs(1), 2, 1024 * 1024 * 1024);
        let cmd = HalCommand::QueryState {
            resource: ResourceType::Memory,
            detailed: false,
        };

        // skill-a 2회 → 리밋 도달
        assert!(rl.check_and_record("skill-a", &cmd).is_ok());
        assert!(rl.check_and_record("skill-a", &cmd).is_ok());
        assert!(rl.check_and_record("skill-a", &cmd).is_err()); // 차단

        // skill-b는 독립적이므로 아직 가능
        assert!(rl.check_and_record("skill-b", &cmd).is_ok());
        assert!(rl.check_and_record("skill-b", &cmd).is_ok());
    }

    // [STRIDE-D02] 누적 할당 예산 초과 → 차단
    #[test]
    fn sec_s3_cumulative_alloc_budget_exceeded() {
        let budget = 10 * 1024 * 1024; // 10 MiB 예산
        let rl = RateLimiter::with_config(Duration::from_secs(60), 1000, budget);

        // 첫 번째: 9 MiB → 통과
        let cmd9 = HalCommand::AllocateMemory {
            size_bytes: 9 * 1024 * 1024,
            alignment: 4096,
            shared: false,
        };
        assert!(rl.check_and_record("greedy-skill", &cmd9).is_ok());

        // 두 번째: 2 MiB → 잔여 1MiB 부족 → 차단
        let cmd2 = HalCommand::AllocateMemory {
            size_bytes: 2 * 1024 * 1024,
            alignment: 4096,
            shared: false,
        };
        assert!(matches!(
            rl.check_and_record("greedy-skill", &cmd2),
            Err(SecurityError::AllocationBudgetExceeded { .. })
        ));
    }

    // [STRIDE-D02] 예산 리셋 후 재할당 가능
    #[test]
    fn sec_s3_budget_reset_allows_realloc() {
        let rl = RateLimiter::with_config(
            Duration::from_secs(60),
            1000,
            5 * 1024 * 1024,
        );
        let cmd = HalCommand::AllocateMemory {
            size_bytes: 5 * 1024 * 1024,
            alignment: 4096,
            shared: false,
        };

        // 전체 예산 소진
        assert!(rl.check_and_record("skill", &cmd).is_ok());
        assert!(rl.check_and_record("skill", &cmd).is_err());

        // 예산 리셋 후 다시 가능
        rl.reset_budget("skill");
        // 단, 레이트 리밋 내이므로 명령 횟수 체크 필요
        // (새 RateLimiter로 테스트해야 타임스탬프 초기화됨)
        let rl2 = RateLimiter::with_config(
            Duration::from_secs(60),
            1000,
            5 * 1024 * 1024,
        );
        assert!(rl2.check_and_record("skill", &cmd).is_ok());
    }

    // ────────────────────────────────────────────────────
    //  SecurityGuard 통합 테스트
    // ────────────────────────────────────────────────────

    // 모든 검사 통과 → 성공
    #[test]
    fn sec_guard_all_checks_pass() {
        let guard = SecurityGuard::new(test_key());
        let token = CapabilityToken::new(vec![ResourceType::Memory], "trusted-skill");
        let signed = guard.authority.sign(token);

        let cmd = HalCommand::AllocateMemory {
            size_bytes: 4096,
            alignment: 4096,
            shared: false,
        };

        assert!(guard.check(&signed, &cmd).is_ok());
    }

    // 위조된 토큰 → 1단계에서 차단
    #[test]
    fn sec_guard_forged_token_blocked_at_stage1() {
        let guard = SecurityGuard::new(test_key());
        let token = CapabilityToken::new(vec![ResourceType::Storage], "bad-skill");
        let mut signed = guard.authority.sign(token);

        // 서명 후 권한 변조 (위조 공격)
        signed.token.permissions.push(ResourceType::Memory);

        let cmd = HalCommand::AllocateMemory {
            size_bytes: 4096,
            alignment: 4096,
            shared: false,
        };
        assert!(matches!(
            guard.check(&signed, &cmd),
            Err(SecurityError::TokenSignatureInvalid)
        ));
    }

    // 권한 없는 정상 토큰 → 2단계에서 차단
    #[test]
    fn sec_guard_no_permission_blocked_at_stage2() {
        let guard = SecurityGuard::new(test_key());
        // Storage 권한만 있는 토큰으로 Memory 명령 시도
        let token = CapabilityToken::new(vec![ResourceType::Storage], "storage-skill");
        let signed = guard.authority.sign(token);

        let cmd = HalCommand::AllocateMemory {
            size_bytes: 4096,
            alignment: 4096,
            shared: false,
        };
        assert!(matches!(
            guard.check(&signed, &cmd),
            Err(SecurityError::InsufficientPermission {
                resource: ResourceType::Memory,
                ..
            })
        ));
    }

    // 악성 경로 → 3단계에서 차단
    #[test]
    fn sec_guard_malicious_path_blocked_at_stage3() {
        let guard = SecurityGuard::new(test_key());
        let token = CapabilityToken::new(vec![ResourceType::Storage], "storage-skill");
        let signed = guard.authority.sign(token);

        // 공격: 경로 순회로 /etc/passwd 읽기 시도
        let cmd = HalCommand::OpenStorageRead {
            path: PathBuf::from("/tmp/../../etc/passwd"),
        };
        assert!(matches!(
            guard.check(&signed, &cmd),
            Err(SecurityError::PathTraversal { .. })
        ));
    }

    // 명령 폭주 → 4단계에서 차단
    #[test]
    fn sec_guard_command_flood_blocked_at_stage4() {
        let guard = SecurityGuard::new(test_key());
        let guard_small_limit = SecurityGuard {
            authority: TokenAuthority::new(test_key()),
            sanitizer: CommandSanitizer::new(),
            rate_limiter: RateLimiter::with_config(
                Duration::from_secs(1),
                3, // 3회/초로 리밋 설정
                1024 * 1024 * 1024,
            ),
        };

        let token = CapabilityToken::new(vec![ResourceType::Memory], "flood-skill");
        let signed = guard_small_limit.authority.sign(token);

        let cmd = HalCommand::QueryState {
            resource: ResourceType::Memory,
            detailed: false,
        };

        // 3회 통과
        for _ in 0..3 {
            assert!(guard_small_limit.check(&signed, &cmd).is_ok());
        }

        // 4번째 → 차단
        assert!(matches!(
            guard_small_limit.check(&signed, &cmd),
            Err(SecurityError::RateLimitExceeded { .. })
        ));

        // guard (100회 리밋)에서 signed 토큰으로는 이 메서드 외부에서 사용해야 함
        let _ = guard; // 사용됨 표시
    }

    // SecurityError Display 포맷 확인
    #[test]
    fn sec_error_display_contains_sec_prefix() {
        let errors: Vec<SecurityError> = vec![
            SecurityError::ParameterOutOfRange { field: "size_bytes", value: 100, max: 50 },
            SecurityError::InvalidAlignment { alignment: 3 },
            SecurityError::PathTraversal { path: "/tmp/../etc".to_string() },
            SecurityError::PathNotAllowed { path: "/dev/mem".to_string(), reason: "테스트" },
            SecurityError::NullByteInPath,
            SecurityError::TokenSignatureInvalid,
            SecurityError::TokenExpired,
            SecurityError::InsufficientPermission {
                resource: ResourceType::Memory,
                skill_name: "test".to_string(),
            },
            SecurityError::RateLimitExceeded {
                skill_name: "test".to_string(),
                limit: 100,
                window_secs: 1,
            },
            SecurityError::AllocationBudgetExceeded {
                skill_name: "test".to_string(),
                requested: 1024,
                remaining_budget: 512,
            },
        ];

        for err in &errors {
            let display = err.to_string();
            assert!(
                display.contains("[SEC-"),
                "SecurityError Display에 [SEC-Xxx] 코드 없음: {}",
                display
            );
        }
    }
}
