// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// This file is part of AIOS (AI-Only Operating System).
//
// App — AIOS Shell 루트 컴포넌트.
//
// 레이아웃 구조:
//   ┌────────────────────────────────────────────────────┐
//   │  TopBar (로고 + 시스템 상태 요약 + 세션 정보)         │
//   ├──────────────┬──────────────────────┬──────────────┤
//   │ SkillList    │  AiShell (메인)       │  HalStatus   │
//   │ (220px)      │  (flex-1)            │  (260px)     │
//   └──────────────┴──────────────────────┴──────────────┘
//
// M0: 모든 패널은 목업 데이터를 사용.
// M1: REST/WebSocket으로 실제 HAL/AI Core 연동.

import { GitBranch, Radio, Shield, Wifi, WifiOff } from "lucide-react";
import { useEffect, useState } from "react";
import { AiShell }   from "./AiShell";
import { HalStatus } from "./HalStatus";
import { SkillList } from "./SkillList";
import { useHalStatus } from "./hooks/useHalStatus";

// ─────────────────────────────────────────────────────────────
//  TopBar — 최상단 상태 표시줄
// ─────────────────────────────────────────────────────────────

/**
 * 시스템 전체 CPU 사용률에 따른 색상.
 */
function cpuStatusColor(usage: number): string {
  if (usage < 0.6) return "text-[#10B981]";
  if (usage < 0.85) return "text-[#F59E0B]";
  return "text-[#EF4444]";
}

/**
 * TopBar
 *
 * 화면 최상단의 시스템 상태 표시줄.
 * 좌측: AIOS 브랜드 + 버전 + 브랜치.
 * 우측: CPU/메모리 요약 + 네트워크 상태 + 보안 배지.
 */
function TopBar() {
  const hal          = useHalStatus();
  const [online, setOnline] = useState(navigator.onLine);

  // 네트워크 상태 구독
  useEffect(() => {
    const onOnline  = () => setOnline(true);
    const onOffline = () => setOnline(false);
    window.addEventListener("online",  onOnline);
    window.addEventListener("offline", onOffline);
    return () => {
      window.removeEventListener("online",  onOnline);
      window.removeEventListener("offline", onOffline);
    };
  }, []);

  const cpuPct = Math.round(hal.cpu.totalUsage  * 100);
  const memPct = Math.round(hal.memory.usageRatio * 100);

  return (
    <header className="flex items-center justify-between px-4 py-2 border-b border-[#1E1E2E] bg-[#0D0D14] shrink-0 select-none">
      {/* 좌측: 브랜드 */}
      <div className="flex items-center gap-3">
        {/* 로고 마크 */}
        <div className="flex items-center gap-1.5">
          <div className="w-5 h-5 rounded-md bg-gradient-to-br from-[#6366F1] to-[#A855F7] flex items-center justify-center">
            <Radio size={11} className="text-white" />
          </div>
          <span className="text-sm font-bold text-[#E2E8F0] tracking-tight">AIOS</span>
          <span className="text-xs text-[#334155] aios-mono">Shell</span>
        </div>

        {/* 버전 + 브랜치 */}
        <div className="flex items-center gap-1.5 border-l border-[#1E1E2E] pl-3">
          <span className="text-[10px] aios-mono text-[#475569]">v0.1.0-M0</span>
          <span className="text-[#1E1E2E]">·</span>
          <GitBranch size={9} className="text-[#334155]" />
          <span className="text-[10px] aios-mono text-[#334155]">main</span>
        </div>
      </div>

      {/* 우측: 상태 표시 */}
      <div className="flex items-center gap-4">
        {/* CPU 요약 */}
        <div className="flex items-center gap-1.5">
          <span className="text-[10px] text-[#475569]">CPU</span>
          <span className={`text-xs font-mono font-semibold ${cpuStatusColor(hal.cpu.totalUsage)}`}>
            {cpuPct}%
          </span>
        </div>

        {/* Memory 요약 */}
        <div className="flex items-center gap-1.5">
          <span className="text-[10px] text-[#475569]">MEM</span>
          <span className={`text-xs font-mono font-semibold ${cpuStatusColor(hal.memory.usageRatio)}`}>
            {memPct}%
          </span>
        </div>

        {/* 구분선 */}
        <div className="w-px h-4 bg-[#1E1E2E]" />

        {/* 네트워크 */}
        <div className="flex items-center gap-1.5">
          {online
            ? <Wifi    size={12} className="text-[#10B981]" />
            : <WifiOff size={12} className="text-[#EF4444]" />
          }
          <span className="text-[10px] text-[#475569]">
            {online ? "온라인" : "오프라인"}
          </span>
        </div>

        {/* 보안 배지 */}
        <div className="flex items-center gap-1 aios-badge bg-[#0a1628] text-[#22D3EE]">
          <Shield size={10} />
          <span className="text-[9px]">SecurityGuard</span>
        </div>
      </div>
    </header>
  );
}

// ─────────────────────────────────────────────────────────────
//  공개 컴포넌트: App
// ─────────────────────────────────────────────────────────────

/**
 * App
 *
 * AIOS Shell 애플리케이션 루트 컴포넌트.
 * TopBar + 3-컬럼 바디(SkillList / AiShell / HalStatus)로 구성.
 *
 * @example
 * // main.tsx에서:
 * <App />
 */
export function App() {
  return (
    <div className="flex flex-col h-full bg-[#0A0A0F] text-[#E2E8F0]">
      {/* 최상단 상태 표시줄 */}
      <TopBar />

      {/* 메인 3-컬럼 레이아웃 */}
      <main className="flex flex-1 min-h-0 overflow-hidden">
        {/* 좌측: 스킬 목록 */}
        <SkillList />

        {/* 중앙: AI 대화 인터페이스 */}
        <AiShell />

        {/* 우측: HAL 실시간 모니터 */}
        <HalStatus />
      </main>
    </div>
  );
}
