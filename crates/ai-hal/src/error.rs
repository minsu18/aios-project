// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! # error
//!
//! AI-OS HAL 전체에서 사용하는 에러 타입 정의 모듈.
//!
//! ## 설계 원칙
//! - 모든 HalError 변형은 AI Core가 재시도/복구 여부를 판단하기 위한
//!   충분한 컨텍스트(리소스 종류, errno, syscall 이름 등)를 포함
//! - `SyscallFailed`: Linux 전용 syscall 에러를 명시적으로 분리하여
//!   플랫폼 비의존 코드와 구분 가능하게 함

use std::fmt;
use std::path::PathBuf;

use crate::ResourceType;

// ─────────────────────────────────────────────
//  HAL 에러 열거형
// ─────────────────────────────────────────────

/// HAL 연산 실패 이유를 나타내는 에러 타입.
///
/// AI Core는 이 에러를 보고 재시도 여부를 결정한다:
/// - `ResourceUnavailable` → 잠시 후 재시도 가능
/// - `PermissionDenied`    → Skill 재등록 필요
/// - `OutOfMemory`         → 더 작은 크기로 재시도
/// - `SyscallFailed`       → 시스템 레벨 문제, 에스컬레이션 필요
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HalError {
    /// capability 토큰에 해당 리소스 접근 권한이 없음.
    ///
    /// AI Core 대응: Skill 재등록 또는 권한 요청
    PermissionDenied {
        /// 접근 거부된 리소스 종류
        resource: ResourceType,
        /// 거부 이유 (로그/감사용)
        reason: String,
    },

    /// 요청한 리소스가 현재 일시적으로 사용 불가.
    ///
    /// AI Core 대응: 일정 시간 후 재시도
    ResourceUnavailable {
        /// 사용 불가한 리소스 종류
        resource: ResourceType,
    },

    /// 요청한 리소스가 이 시스템에 존재하지 않음 (영구적).
    ///
    /// AI Core 대응: 다른 리소스 유형 시도
    ResourceNotFound {
        /// 존재하지 않는 리소스 종류
        resource: ResourceType,
    },

    /// 메모리 부족으로 할당 실패.
    ///
    /// AI Core 대응: 더 작은 크기로 분할 요청
    OutOfMemory {
        /// 요청한 바이트 수
        requested_bytes: usize,
        /// 현재 사용 가능한 바이트 수 (알 수 있는 경우)
        available_bytes: Option<usize>,
    },

    /// 스토리지 경로 접근 또는 I/O 실패.
    ///
    /// AI Core 대응: 경로 확인 후 재시도
    StoragePathError {
        /// 접근 실패한 경로
        path: PathBuf,
        /// OS 에러 메시지
        os_error: String,
    },

    /// 유효하지 않은 명령 파라미터.
    ///
    /// AI Core 대응: Intent 재파싱 후 다른 파라미터로 재시도
    InvalidParameter {
        /// 잘못된 파라미터 이름
        param_name: String,
        /// 에러 상세 설명
        message: String,
    },

    /// Linux syscall 호출 실패.
    ///
    /// 플랫폼 전용 에러: POSIX syscall이 음수 값을 반환한 경우.
    /// AI Core 대응: errno 확인 후 판단 (EAGAIN → 재시도, EPERM → 에스컬레이션)
    ///
    /// 참조: https://man7.org/linux/man-pages/man3/errno.3.html
    SyscallFailed {
        /// 실패한 syscall 이름 (예: "mmap", "sched_setaffinity", "statfs")
        syscall: &'static str,
        /// POSIX errno 값
        /// 예: 1 = EPERM, 12 = ENOMEM, 13 = EACCES, 11 = EAGAIN
        errno: i32,
        /// strerror 기반 에러 메시지 (human-readable)
        message: String,
    },

    /// HAL 내부 버그 또는 예상치 못한 상태.
    ///
    /// AI Core 대응: 로그 기록 후 에스컬레이션
    InternalError(String),
}

impl HalError {
    /// 이 에러가 일시적(재시도 가능)인지 여부 반환.
    ///
    /// AI Core의 재시도 로직에서 사용.
    ///
    /// 재시도 가능: ResourceUnavailable, SyscallFailed(EAGAIN/EINTR)
    /// 재시도 불가: PermissionDenied, ResourceNotFound, InvalidParameter
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ResourceUnavailable { .. } => true,
            // EAGAIN(11) 또는 EINTR(4): 일시적 실패, 재시도 가능
            Self::SyscallFailed { errno, .. } => matches!(errno, 11 | 4),
            _ => false,
        }
    }

    /// 이 에러가 보안 관련(권한) 에러인지 여부 반환.
    ///
    /// 보안 감사 로그에서 별도 처리할 때 사용.
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            Self::PermissionDenied { .. }
                | Self::SyscallFailed { errno: 1 | 13, .. } // EPERM(1), EACCES(13)
        )
    }

    /// 현재 errno에 해당하는 POSIX 에러 이름 문자열 반환.
    ///
    /// SyscallFailed 변형에서만 의미 있음.
    pub fn errno_name(&self) -> Option<&'static str> {
        if let Self::SyscallFailed { errno, .. } = self {
            Some(errno_to_name(*errno))
        } else {
            None
        }
    }
}

impl fmt::Display for HalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied { resource, reason } => {
                write!(f, "권한 거부: {} — {}", resource, reason)
            }
            Self::ResourceUnavailable { resource } => {
                write!(f, "리소스 일시 사용 불가: {}", resource)
            }
            Self::ResourceNotFound { resource } => {
                write!(f, "리소스 없음: {}", resource)
            }
            Self::OutOfMemory {
                requested_bytes,
                available_bytes,
            } => match available_bytes {
                Some(avail) => write!(
                    f,
                    "메모리 부족: {}바이트 요청, {}바이트만 사용 가능",
                    requested_bytes, avail
                ),
                None => write!(f, "메모리 부족: {}바이트 요청", requested_bytes),
            },
            Self::StoragePathError { path, os_error } => {
                write!(f, "스토리지 오류 ({:?}): {}", path, os_error)
            }
            Self::InvalidParameter { param_name, message } => {
                write!(f, "잘못된 파라미터 '{}': {}", param_name, message)
            }
            Self::SyscallFailed {
                syscall,
                errno,
                message,
            } => {
                write!(
                    f,
                    "syscall 실패 {}(): errno={} ({}) — {}",
                    syscall,
                    errno,
                    errno_to_name(*errno),
                    message
                )
            }
            Self::InternalError(msg) => write!(f, "HAL 내부 오류: {}", msg),
        }
    }
}

impl std::error::Error for HalError {}

// ─────────────────────────────────────────────
//  errno → 이름 변환 헬퍼
// ─────────────────────────────────────────────

/// errno 숫자를 POSIX 표준 이름 문자열로 변환.
///
/// 참조: https://man7.org/linux/man-pages/man3/errno.3.html
fn errno_to_name(errno: i32) -> &'static str {
    match errno {
        1 => "EPERM",        // 허용되지 않는 작업
        2 => "ENOENT",       // 파일/디렉토리 없음
        4 => "EINTR",        // 시그널로 인한 인터럽트
        5 => "EIO",          // I/O 에러
        11 => "EAGAIN",      // 다시 시도
        12 => "ENOMEM",      // 메모리 부족
        13 => "EACCES",      // 권한 거부
        14 => "EFAULT",      // 잘못된 메모리 주소
        16 => "EBUSY",       // 장치/리소스 사용 중
        17 => "EEXIST",      // 이미 존재함
        22 => "EINVAL",      // 유효하지 않은 인자
        24 => "EMFILE",      // 너무 많은 열린 파일
        28 => "ENOSPC",      // 장치에 공간 없음
        38 => "ENOSYS",      // 함수 미구현
        95 => "EOPNOTSUPP",  // 지원하지 않는 작업
        _ => "EUNKNOWN",
    }
}

// ─────────────────────────────────────────────
//  편의 변환: std::io::Error → HalError
// ─────────────────────────────────────────────

impl From<std::io::Error> for HalError {
    /// std::io::Error를 HalError::SyscallFailed로 변환.
    ///
    /// 파일 I/O 실패를 HAL 에러 체계로 통합할 때 사용.
    fn from(e: std::io::Error) -> Self {
        let errno = e.raw_os_error().unwrap_or(0);
        Self::SyscallFailed {
            syscall: "io",
            errno,
            message: e.to_string(),
        }
    }
}

// ─────────────────────────────────────────────
//  단위 테스트
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_denied_display() {
        // 권한 거부 에러 메시지 한국어 출력 확인
        let err = HalError::PermissionDenied {
            resource: ResourceType::Memory,
            reason: "테스트 거부".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("권한 거부"));
        assert!(msg.contains("Memory"));
        assert!(msg.contains("테스트 거부"));
    }

    #[test]
    fn test_out_of_memory_with_available() {
        // 사용 가능 용량 포함된 OOM 에러 메시지 확인
        let err = HalError::OutOfMemory {
            requested_bytes: 1024,
            available_bytes: Some(512),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1024"));
        assert!(msg.contains("512"));
    }

    #[test]
    fn test_syscall_failed_display_with_errno_name() {
        // syscall 실패 에러가 errno 이름을 포함하는지 확인
        let err = HalError::SyscallFailed {
            syscall: "mmap",
            errno: 12, // ENOMEM
            message: "메모리 부족으로 mmap 실패".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("mmap"));
        assert!(msg.contains("ENOMEM"));
        assert!(msg.contains("12"));
    }

    #[test]
    fn test_is_retryable() {
        // EAGAIN(11) → 재시도 가능
        let eagain = HalError::SyscallFailed {
            syscall: "read",
            errno: 11,
            message: String::new(),
        };
        assert!(eagain.is_retryable());

        // EPERM(1) → 재시도 불가
        let eperm = HalError::SyscallFailed {
            syscall: "mmap",
            errno: 1,
            message: String::new(),
        };
        assert!(!eperm.is_retryable());

        // ResourceUnavailable → 재시도 가능
        let unavail = HalError::ResourceUnavailable {
            resource: ResourceType::Gpu,
        };
        assert!(unavail.is_retryable());

        // PermissionDenied → 재시도 불가
        let denied = HalError::PermissionDenied {
            resource: ResourceType::Memory,
            reason: String::new(),
        };
        assert!(!denied.is_retryable());
    }

    #[test]
    fn test_is_security_error() {
        // PermissionDenied → 보안 에러
        let denied = HalError::PermissionDenied {
            resource: ResourceType::Storage,
            reason: String::new(),
        };
        assert!(denied.is_security_error());

        // EACCES(13) → 보안 에러
        let eacces = HalError::SyscallFailed {
            syscall: "open",
            errno: 13,
            message: String::new(),
        };
        assert!(eacces.is_security_error());

        // OOM → 보안 에러 아님
        let oom = HalError::OutOfMemory {
            requested_bytes: 1024,
            available_bytes: None,
        };
        assert!(!oom.is_security_error());
    }

    #[test]
    fn test_errno_name() {
        let err = HalError::SyscallFailed {
            syscall: "test",
            errno: 1,
            message: String::new(),
        };
        assert_eq!(err.errno_name(), Some("EPERM"));

        // SyscallFailed 아닌 경우 None 반환
        let other = HalError::InternalError("test".to_string());
        assert_eq!(other.errno_name(), None);
    }

    #[test]
    fn test_from_io_error() {
        // std::io::Error → HalError 변환 테스트
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "테스트");
        let hal_err = HalError::from(io_err);
        assert!(matches!(hal_err, HalError::SyscallFailed { .. }));
    }
}
