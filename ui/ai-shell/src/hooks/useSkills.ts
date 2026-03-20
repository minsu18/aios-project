// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// useSkills — 설치된 스킬 목록 관리 훅
// M0: 하드코딩된 모의 스킬 데이터 사용.
// M1: GET /api/skills 엔드포인트 연동 + WebSocket 실시간 갱신 예정.

import { useCallback, useState } from "react";
import type { Skill } from "../types";

// ── M0 모의 스킬 데이터 ──────────────────────────────────────
// 실제 SKILL.md 스펙을 따르는 예시 스킬들.
// 향후 skills/ 디렉토리를 스캔하는 API로 대체됨.

const MOCK_SKILLS: Skill[] = [
  {
    id:          "example",
    name:        "example",
    description: "기본 예제 스킬 — 시간 조회 및 텍스트 에코",
    version:     "0.1.0",
    status:      "active",
    installPath: ".aios/skills/example",
    lastUsedAt:  Date.now() - 1_200_000,
    tools: [
      {
        name:        "get_time",
        description: "현재 로컬 시각 반환 (IANA 타임존 지원)",
      },
      {
        name:        "echo",
        description: "입력 텍스트를 그대로 반환 (디버그용)",
      },
    ],
  },
  {
    id:          "memory-manager",
    name:        "memory-manager",
    description: "HAL 메모리 블록 할당/해제 및 사용률 모니터링",
    version:     "0.2.1",
    status:      "active",
    installPath: ".aios/skills/memory-manager",
    lastUsedAt:  Date.now() - 300_000,
    tools: [
      {
        name:        "alloc",
        description: "지정한 크기의 메모리 블록 할당",
      },
      {
        name:        "free",
        description: "할당된 메모리 블록 해제",
      },
      {
        name:        "query",
        description: "전체 / 사용 / 여유 메모리 현황 조회",
      },
    ],
  },
  {
    id:          "cpu-scheduler",
    name:        "cpu-scheduler",
    description: "CPU 어피니티 힌트 및 우선순위 제어",
    version:     "0.1.3",
    status:      "active",
    installPath: ".aios/skills/cpu-scheduler",
    lastUsedAt:  Date.now() - 5_400_000,
    tools: [
      {
        name:        "set_affinity",
        description: "프로세스를 특정 코어에 고정",
      },
      {
        name:        "get_affinity",
        description: "현재 어피니티 마스크 조회",
      },
      {
        name:        "set_priority",
        description: "태스크 스케줄링 우선순위 설정",
      },
    ],
  },
  {
    id:          "file-ops",
    name:        "file-ops",
    description: "HAL 파일 시스템 — 읽기/쓰기/메타데이터 조작",
    version:     "0.3.0",
    status:      "inactive",
    installPath: ".aios/skills/file-ops",
    tools: [
      {
        name:        "open",
        description: "파일 디스크립터 열기",
      },
      {
        name:        "read",
        description: "파일에서 데이터 읽기",
      },
      {
        name:        "write",
        description: "파일에 데이터 쓰기",
      },
      {
        name:        "close",
        description: "파일 디스크립터 닫기",
      },
    ],
  },
  {
    id:          "inference-router",
    name:        "inference-router",
    description: "온디바이스 / 클라우드 추론 백엔드 라우팅 제어",
    version:     "0.1.0",
    status:      "loading",
    installPath: ".aios/skills/inference-router",
    tools: [
      {
        name:        "route",
        description: "신뢰도 기반 백엔드 선택 (rule → ondevice → cloud)",
      },
      {
        name:        "stats",
        description: "라우터 통계 조회 (백엔드별 호출 횟수)",
      },
    ],
  },
];

// ── 공개 훅 ──────────────────────────────────────────────────

/** useSkills 반환 타입 */
export interface UseSkillsResult {
  /** 전체 스킬 목록 */
  skills:       Skill[];
  /** 특정 스킬 활성/비활성 토글 */
  toggleSkill:  (id: string) => void;
  /** 활성 스킬 수 */
  activeCount:  number;
}

/**
 * useSkills
 *
 * 설치된 AIOS 스킬 목록과 활성화 상태를 관리하는 훅.
 * 스킬 토글 시 낙관적 업데이트(optimistic update)를 수행하며,
 * M1에서는 실제 API 호출 후 상태 동기화로 전환됨.
 *
 * @example
 * const { skills, toggleSkill, activeCount } = useSkills();
 */
export function useSkills(): UseSkillsResult {
  const [skills, setSkills] = useState<Skill[]>(MOCK_SKILLS);

  /**
   * 특정 스킬의 active/inactive 상태를 토글.
   * loading/error 상태의 스킬은 토글 불가.
   */
  const toggleSkill = useCallback((id: string) => {
    setSkills((prev) =>
      prev.map((skill) => {
        if (skill.id !== id) return skill;
        // loading / error 상태는 토글 불가
        if (skill.status === "loading" || skill.status === "error") return skill;
        return {
          ...skill,
          status:     skill.status === "active" ? "inactive" : "active",
          lastUsedAt: skill.status === "inactive" ? Date.now() : skill.lastUsedAt,
        };
      })
    );
  }, []);

  const activeCount = skills.filter((s) => s.status === "active").length;

  return { skills, toggleSkill, activeCount };
}
