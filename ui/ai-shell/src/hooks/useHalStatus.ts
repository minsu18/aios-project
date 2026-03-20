// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// useHalStatus — HAL 상태 폴링 훅
// M0: 로컬 시뮬레이션 데이터 반환.
// M1: WebSocket / REST /api/hal/status 엔드포인트로 교체 예정.

import { useEffect, useRef, useState } from "react";
import type { HalState } from "../types";

// ── 상수 ─────────────────────────────────────────────────────

/** 폴링 간격 (ms) */
const POLL_INTERVAL_MS = 1_000;

/** 시뮬레이션 베이스 값 — Raspberry Pi 4B 기준 */
const BASE: HalState = {
  cpu: {
    totalUsage:   0.18,
    perCoreUsage: [0.14, 0.22, 0.09, 0.27],
    modelName:    "Cortex-A72 @ 1.80 GHz",
    coreCount:    4,
    loadAvg:      [0.42, 0.38, 0.31],
  },
  memory: {
    totalBytes:     4_294_967_296, // 4 GiB
    usedBytes:      1_073_741_824, // 1 GiB
    availableBytes: 3_221_225_472, // 3 GiB
    usageRatio:     0.25,
  },
  inference: {
    backend:      "ondevice",
    modelName:    "Phi-4-mini (Q4_K_M)",
    tokensPerSec: 12.4,
    isRunning:    false,
    contextUsage: 0.08,
  },
  storage: {
    totalBytes: 64_424_509_440, // 60 GiB
    usedBytes:  11_274_289_152, // ~10.5 GiB
    usageRatio: 0.175,
    mountPoint: "/",
    fsType:     "ext4",
  },
  updatedAt: Date.now(),
};

// ── 내부 유틸 ────────────────────────────────────────────────

/**
 * [-range, +range] 내 무작위 섭동(perturbation) 생성.
 * 실제 HAL 폴링을 흉내내기 위한 시뮬레이션용.
 */
function jitter(base: number, range: number, min = 0, max = 1): number {
  return Math.min(max, Math.max(min, base + (Math.random() * 2 - 1) * range));
}

/**
 * 현재 HAL 상태 스냅샷 생성 (시뮬레이션).
 * M1에서는 REST fetch() 또는 WebSocket 메시지로 교체됨.
 */
function simulateHalState(prev: HalState): HalState {
  const cpuTotal = jitter(prev.cpu.totalUsage, 0.05);
  const memRatio = jitter(prev.memory.usageRatio, 0.01);

  return {
    cpu: {
      ...prev.cpu,
      totalUsage:   cpuTotal,
      perCoreUsage: prev.cpu.perCoreUsage.map((u) => jitter(u, 0.07)),
      loadAvg:      [
        jitter(prev.cpu.loadAvg[0], 0.03, 0),
        jitter(prev.cpu.loadAvg[1], 0.02, 0),
        jitter(prev.cpu.loadAvg[2], 0.01, 0),
      ],
    },
    memory: {
      ...prev.memory,
      usageRatio:     memRatio,
      usedBytes:      Math.round(prev.memory.totalBytes * memRatio),
      availableBytes: Math.round(prev.memory.totalBytes * (1 - memRatio)),
    },
    inference: {
      ...prev.inference,
      tokensPerSec: jitter(prev.inference.tokensPerSec, 1.5, 0, 200),
    },
    storage: {
      ...prev.storage,
      // 스토리지는 느리게 변함
      usageRatio: jitter(prev.storage.usageRatio, 0.001, 0, 1),
    },
    updatedAt: Date.now(),
  };
}

// ── 공개 훅 ──────────────────────────────────────────────────

/**
 * useHalStatus
 *
 * HAL(Hardware Abstraction Layer) 상태를 주기적으로 조회하는 React 훅.
 * 반환값의 모든 필드는 읽기 전용이며, 컴포넌트 재렌더링에 안전.
 *
 * @param intervalMs - 폴링 간격 (기본: 1000ms)
 * @returns HalState 스냅샷 (매 intervalMs마다 갱신)
 *
 * @example
 * const hal = useHalStatus();
 * console.log(`CPU: ${(hal.cpu.totalUsage * 100).toFixed(1)}%`);
 */
export function useHalStatus(intervalMs = POLL_INTERVAL_MS): HalState {
  const [state, setState] = useState<HalState>(BASE);
  // 최신 상태를 인터벌 클로저에서 참조하기 위한 ref
  const stateRef = useRef<HalState>(state);
  stateRef.current = state;

  useEffect(() => {
    const id = setInterval(() => {
      setState(simulateHalState(stateRef.current));
    }, intervalMs);

    return () => clearInterval(id);
  }, [intervalMs]);

  return state;
}
