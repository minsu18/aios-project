// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// This file is part of AIOS (AI-Only Operating System).
//
// AIOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// ─────────────────────────────────────────────────────────────
//  AIOS UI 공유 타입 정의
//  - AI 대화, HAL 상태, 스킬 메타데이터에 관한 모든 타입을 포함
// ─────────────────────────────────────────────────────────────

// ── 대화 메시지 ──────────────────────────────────────────────

/** 메시지 역할: 사용자 / AI 어시스턴트 / 시스템 알림 */
export type MessageRole = "user" | "assistant" | "system";

/** HAL 명령 실행 상태 */
export type HalCommandStatus = "pending" | "executing" | "done" | "error";

/** HAL 명령 타입 (intent_engine/models.py 의 HalCommandType 미러) */
export type HalCommandType =
  | "QUERY_STATE"
  | "ALLOC_MEM"
  | "FREE_MEM"
  | "CPU_HINT"
  | "OPEN_FILE"
  | "WRITE_FILE"
  | "REGISTER_SKILL";

/** AI가 생성한 HAL 명령 한 건 */
export interface HalCommand {
  /** 명령 고유 ID */
  id:      string;
  /** HAL 명령 종류 */
  type:    HalCommandType;
  /** 대상 리소스 (cpu | memory | storage | skill) */
  resource: string;
  /** 명령 파라미터 */
  params:  Record<string, unknown>;
  /** 실행 상태 */
  status:  HalCommandStatus;
}

/** 대화 메시지 단위 */
export interface Message {
  /** 메시지 고유 ID (crypto.randomUUID) */
  id:          string;
  /** 발신 역할 */
  role:        MessageRole;
  /** 텍스트 본문 */
  content:     string;
  /** 생성 시각 (Unix ms) */
  timestamp:   number;
  /** AI가 생성한 HAL 명령 목록 (어시스턴트 메시지에만 존재) */
  halCommands?: HalCommand[];
  /** 스트리밍 중 여부 (타이핑 애니메이션용) */
  isStreaming?: boolean;
}

// ── HAL 상태 ─────────────────────────────────────────────────

/** CPU 상태 스냅샷 */
export interface CpuState {
  /** 전체 사용률 (0.0 ~ 1.0) */
  totalUsage:    number;
  /** 코어별 사용률 배열 */
  perCoreUsage:  number[];
  /** CPU 모델명 */
  modelName:     string;
  /** 논리 코어 수 */
  coreCount:     number;
  /** 부하 평균 (1m/5m/15m) */
  loadAvg:       [number, number, number];
}

/** 메모리 상태 스냅샷 */
export interface MemoryState {
  /** 전체 메모리 (bytes) */
  totalBytes:     number;
  /** 사용 중 메모리 (bytes) */
  usedBytes:      number;
  /** 사용 가능 메모리 (bytes) */
  availableBytes: number;
  /** 사용률 (0.0 ~ 1.0) */
  usageRatio:     number;
}

/** 추론 (AI) 상태 */
export interface InferenceState {
  /** 사용 중인 백엔드 */
  backend:     "rule" | "ondevice" | "cloud";
  /** 모델명 */
  modelName:   string;
  /** 초당 토큰 생성 속도 */
  tokensPerSec: number;
  /** 현재 추론 실행 중 여부 */
  isRunning:   boolean;
  /** 컨텍스트 윈도우 사용률 (0.0 ~ 1.0) */
  contextUsage: number;
}

/** 스토리지 상태 */
export interface StorageState {
  /** 전체 용량 (bytes) */
  totalBytes: number;
  /** 사용 용량 (bytes) */
  usedBytes:  number;
  /** 사용률 (0.0 ~ 1.0) */
  usageRatio: number;
  /** 마운트 포인트 */
  mountPoint: string;
  /** 파일시스템 종류 */
  fsType:     string;
}

/** 전체 HAL 상태 (HalStatus 패널에서 사용) */
export interface HalState {
  cpu:       CpuState;
  memory:    MemoryState;
  inference: InferenceState;
  storage:   StorageState;
  /** 마지막 갱신 시각 (Unix ms) */
  updatedAt: number;
}

// ── 스킬 ─────────────────────────────────────────────────────

/** 스킬 툴 정의 (SKILL.md tools 필드 미러) */
export interface SkillTool {
  name:        string;
  description: string;
}

/** 스킬 활성화 상태 */
export type SkillStatus = "active" | "inactive" | "loading" | "error";

/** 설치된 스킬 메타데이터 */
export interface Skill {
  /** 스킬 고유 ID (name 기반) */
  id:          string;
  /** 스킬 이름 (SKILL.md name 필드) */
  name:        string;
  /** 스킬 설명 */
  description: string;
  /** 버전 (semver) */
  version:     string;
  /** 노출 툴 목록 */
  tools:       SkillTool[];
  /** 활성화 상태 */
  status:      SkillStatus;
  /** 설치 경로 */
  installPath: string;
  /** 마지막 사용 시각 (Unix ms, optional) */
  lastUsedAt?: number;
}
