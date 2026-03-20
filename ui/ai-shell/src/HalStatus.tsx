// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// This file is part of AIOS (AI-Only Operating System).
//
// HalStatus — CPU / Memory / Inference / Storage 실시간 상태 패널.
// useHalStatus 훅으로부터 1초마다 갱신되는 HalState를 받아 렌더링.

import { Activity, Brain, Cpu, Database, HardDrive } from "lucide-react";
import { useHalStatus } from "./hooks/useHalStatus";
import type { HalState } from "./types";

// ─────────────────────────────────────────────────────────────
//  내부 공통 컴포넌트
// ─────────────────────────────────────────────────────────────

/** 게이지 바 색상: 사용률에 따라 green → yellow → red */
function gaugeColor(ratio: number): string {
  if (ratio < 0.6) return "bg-[#10B981]"; // 정상
  if (ratio < 0.85) return "bg-[#F59E0B]"; // 경고
  return "bg-[#EF4444]";                   // 위험
}

/** 사용률 텍스트 색상 */
function ratioTextColor(ratio: number): string {
  if (ratio < 0.6) return "text-[#10B981]";
  if (ratio < 0.85) return "text-[#F59E0B]";
  return "text-[#EF4444]";
}

/** bytes → 사람이 읽기 쉬운 단위 (GiB / MiB / KiB) */
function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GiB`;
  if (bytes >= 1_048_576)     return `${(bytes / 1_048_576).toFixed(0)} MiB`;
  if (bytes >= 1_024)         return `${(bytes / 1_024).toFixed(0)} KiB`;
  return `${bytes} B`;
}

// ── 섹션 헤더 ─────────────────────────────────────────────

interface SectionHeaderProps {
  icon:  React.ReactNode;
  label: string;
}

/**
 * 패널 섹션 제목 행 (아이콘 + 레이블).
 */
function SectionHeader({ icon, label }: SectionHeaderProps) {
  return (
    <div className="flex items-center gap-2 mb-3">
      <span className="text-[#6366F1]">{icon}</span>
      <span className="text-xs font-semibold text-[#94A3B8] uppercase tracking-widest">
        {label}
      </span>
    </div>
  );
}

// ── 단일 게이지 행 ─────────────────────────────────────────

interface GaugeRowProps {
  label:  string;
  ratio:  number;
  detail: string;
}

/**
 * 레이블 + 게이지 바 + 퍼센트 한 줄 컴포넌트.
 * @param label  - 좌측 표시 이름
 * @param ratio  - 0.0 ~ 1.0 사용률
 * @param detail - 우측 보조 텍스트
 */
function GaugeRow({ label, ratio, detail }: GaugeRowProps) {
  const pct = Math.round(ratio * 100);
  return (
    <div className="mb-3 last:mb-0">
      {/* 헤더 행 */}
      <div className="flex justify-between items-baseline mb-1">
        <span className="text-xs text-[#64748B]">{label}</span>
        <span className={`text-xs font-mono font-medium ${ratioTextColor(ratio)}`}>
          {pct}%
        </span>
      </div>
      {/* 게이지 트랙 */}
      <div className="h-1.5 bg-[#1E1E2E] rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full gauge-bar ${gaugeColor(ratio)}`}
          style={{ width: `${pct}%` }}
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={label}
        />
      </div>
      {/* 보조 정보 */}
      <div className="mt-0.5 text-[10px] text-[#475569] aios-mono">{detail}</div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
//  서브 패널: CPU
// ─────────────────────────────────────────────────────────────

/**
 * CPU 상태 서브 패널.
 * 총 사용률 게이지 + 코어별 미니 바 + 로드 에버리지 표시.
 */
function CpuPanel({ hal }: { hal: HalState }) {
  const { cpu } = hal;
  const pct = Math.round(cpu.totalUsage * 100);

  return (
    <section className="aios-card p-4 mb-3">
      <SectionHeader icon={<Cpu size={14} />} label="CPU" />

      {/* 전체 사용률 */}
      <GaugeRow
        label="Total Usage"
        ratio={cpu.totalUsage}
        detail={cpu.modelName}
      />

      {/* 코어별 미니 바 */}
      <div className="grid grid-cols-4 gap-1 mt-3">
        {cpu.perCoreUsage.map((usage, i) => (
          <div key={i} className="text-center">
            <div className="h-10 bg-[#1E1E2E] rounded relative overflow-hidden">
              <div
                className={`absolute bottom-0 left-0 right-0 rounded gauge-bar ${gaugeColor(usage)}`}
                style={{ height: `${Math.round(usage * 100)}%` }}
              />
            </div>
            <div className="text-[9px] text-[#475569] mt-0.5 aios-mono">
              C{i}
            </div>
          </div>
        ))}
      </div>

      {/* 로드 에버리지 */}
      <div className="mt-3 flex gap-3">
        {(["1m", "5m", "15m"] as const).map((label, i) => (
          <div key={label} className="flex-1 text-center">
            <div className="text-[10px] text-[#475569]">{label}</div>
            <div className={`text-xs font-mono font-medium ${ratioTextColor(cpu.loadAvg[i] / cpu.coreCount)}`}>
              {cpu.loadAvg[i].toFixed(2)}
            </div>
          </div>
        ))}
      </div>

      {/* 사용률 숫자 강조 표시 */}
      <div className="mt-3 text-center">
        <span className={`text-2xl font-mono font-bold ${ratioTextColor(cpu.totalUsage)}`}>
          {pct}
        </span>
        <span className="text-xs text-[#64748B] ml-0.5">%</span>
      </div>
    </section>
  );
}

// ─────────────────────────────────────────────────────────────
//  서브 패널: Memory
// ─────────────────────────────────────────────────────────────

/**
 * 메모리 상태 서브 패널.
 * 사용률 게이지 + 절대량(used/total) 표시.
 */
function MemoryPanel({ hal }: { hal: HalState }) {
  const { memory } = hal;
  return (
    <section className="aios-card p-4 mb-3">
      <SectionHeader icon={<Database size={14} />} label="Memory" />
      <GaugeRow
        label="Used"
        ratio={memory.usageRatio}
        detail={`${formatBytes(memory.usedBytes)} / ${formatBytes(memory.totalBytes)}`}
      />
      <div className="flex justify-between mt-2">
        <div>
          <div className="text-[10px] text-[#475569]">Available</div>
          <div className="text-xs font-mono text-[#10B981]">
            {formatBytes(memory.availableBytes)}
          </div>
        </div>
        <div className="text-right">
          <div className="text-[10px] text-[#475569]">Used</div>
          <div className={`text-xs font-mono ${ratioTextColor(memory.usageRatio)}`}>
            {formatBytes(memory.usedBytes)}
          </div>
        </div>
      </div>
    </section>
  );
}

// ─────────────────────────────────────────────────────────────
//  서브 패널: Inference (AI)
// ─────────────────────────────────────────────────────────────

/** 백엔드 이름 한국어 레이블 */
const BACKEND_LABEL: Record<string, string> = {
  rule:     "규칙 기반",
  ondevice: "온디바이스",
  cloud:    "클라우드",
};

/** 백엔드 배지 색상 */
const BACKEND_COLOR: Record<string, string> = {
  rule:     "bg-[#1E1E2E] text-[#94A3B8]",
  ondevice: "bg-[#1a1f3d] text-[#6366F1]",
  cloud:    "bg-[#1a2d3d] text-[#22D3EE]",
};

/**
 * AI 추론 상태 서브 패널.
 * 백엔드 배지, 컨텍스트 사용률, tok/s 표시.
 */
function InferencePanel({ hal }: { hal: HalState }) {
  const { inference } = hal;
  return (
    <section className="aios-card p-4 mb-3">
      <SectionHeader icon={<Brain size={14} />} label="Inference" />

      {/* 모델명 + 백엔드 배지 */}
      <div className="flex items-center justify-between mb-3">
        <span className="text-xs text-[#94A3B8] aios-mono truncate max-w-[120px]">
          {inference.modelName}
        </span>
        <span className={`aios-badge text-[10px] ${BACKEND_COLOR[inference.backend]}`}>
          {BACKEND_LABEL[inference.backend] ?? inference.backend}
        </span>
      </div>

      {/* 컨텍스트 사용률 */}
      <GaugeRow
        label="Context Window"
        ratio={inference.contextUsage}
        detail={`${Math.round(inference.contextUsage * 128_000).toLocaleString()} / 128k tokens`}
      />

      {/* tok/s + 실행 상태 */}
      <div className="flex items-center justify-between mt-3">
        <div>
          <div className="text-[10px] text-[#475569]">Throughput</div>
          <div className="text-sm font-mono font-semibold text-[#22D3EE]">
            {inference.tokensPerSec.toFixed(1)}
            <span className="text-[10px] text-[#475569] ml-1">tok/s</span>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <div className={`status-dot ${inference.isRunning ? "bg-[#A855F7] animate-pulse" : "bg-[#1E1E2E]"}`} />
          <span className="text-[10px] text-[#64748B]">
            {inference.isRunning ? "추론 중" : "대기"}
          </span>
        </div>
      </div>
    </section>
  );
}

// ─────────────────────────────────────────────────────────────
//  서브 패널: Storage
// ─────────────────────────────────────────────────────────────

/**
 * 스토리지 상태 서브 패널.
 */
function StoragePanel({ hal }: { hal: HalState }) {
  const { storage } = hal;
  return (
    <section className="aios-card p-4">
      <SectionHeader icon={<HardDrive size={14} />} label="Storage" />
      <GaugeRow
        label={`${storage.mountPoint} (${storage.fsType})`}
        ratio={storage.usageRatio}
        detail={`${formatBytes(storage.usedBytes)} / ${formatBytes(storage.totalBytes)}`}
      />
    </section>
  );
}

// ─────────────────────────────────────────────────────────────
//  공개 컴포넌트: HalStatus
// ─────────────────────────────────────────────────────────────

/**
 * HalStatus
 *
 * CPU / Memory / Inference / Storage 상태를 1초마다 갱신하여
 * 표시하는 우측 사이드바 패널 컴포넌트.
 *
 * @example
 * <HalStatus />
 */
export function HalStatus() {
  const hal = useHalStatus();

  return (
    <aside className="w-[260px] shrink-0 flex flex-col h-full aios-panel border-l overflow-y-auto">
      {/* 패널 헤더 */}
      <div className="flex items-center gap-2 px-4 py-3 border-b border-[#1E1E2E] shrink-0">
        <Activity size={14} className="text-[#6366F1]" />
        <span className="text-xs font-semibold text-[#94A3B8] uppercase tracking-widest">
          HAL Monitor
        </span>
        {/* 마지막 갱신 시각 */}
        <span className="ml-auto text-[9px] text-[#334155] aios-mono">
          {new Date(hal.updatedAt).toLocaleTimeString("ko-KR", {
            hour:   "2-digit",
            minute: "2-digit",
            second: "2-digit",
          })}
        </span>
      </div>

      {/* 섹션들 */}
      <div className="flex-1 overflow-y-auto p-3 space-y-0">
        <CpuPanel       hal={hal} />
        <MemoryPanel    hal={hal} />
        <InferencePanel hal={hal} />
        <StoragePanel   hal={hal} />
      </div>
    </aside>
  );
}
