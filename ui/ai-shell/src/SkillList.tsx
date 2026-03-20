// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// This file is part of AIOS (AI-Only Operating System).
//
// SkillList — 설치된 AIOS 스킬 목록 사이드바.
// 각 스킬의 상태(active/inactive/loading/error), 툴 수, 설명을 표시하고
// 토글 버튼으로 활성화/비활성화 가능.

import { ChevronDown, ChevronRight, Package, Wrench, Zap } from "lucide-react";
import { useState } from "react";
import { useSkills } from "./hooks/useSkills";
import type { Skill, SkillStatus } from "./types";

// ─────────────────────────────────────────────────────────────
//  내부 유틸
// ─────────────────────────────────────────────────────────────

/** 상태 도트 색상 */
const STATUS_DOT_COLOR: Record<SkillStatus, string> = {
  active:   "bg-[#10B981]",
  inactive: "bg-[#1E1E2E]",
  loading:  "bg-[#F59E0B] animate-pulse-slow",
  error:    "bg-[#EF4444]",
};

/** 상태 한국어 레이블 */
const STATUS_LABEL: Record<SkillStatus, string> = {
  active:   "활성",
  inactive: "비활성",
  loading:  "로딩 중",
  error:    "오류",
};

/** 마지막 사용 시각 상대 포맷 */
function relativeTime(ts?: number): string {
  if (!ts) return "미사용";
  const diffMs = Date.now() - ts;
  const mins   = Math.floor(diffMs / 60_000);
  const hours  = Math.floor(diffMs / 3_600_000);
  if (mins < 1)   return "방금 전";
  if (mins < 60)  return `${mins}분 전`;
  if (hours < 24) return `${hours}시간 전`;
  return `${Math.floor(hours / 24)}일 전`;
}

// ─────────────────────────────────────────────────────────────
//  SkillCard — 스킬 1개 카드
// ─────────────────────────────────────────────────────────────

interface SkillCardProps {
  skill:        Skill;
  onToggle:     (id: string) => void;
}

/**
 * SkillCard
 *
 * 단일 스킬의 이름, 버전, 설명, 도구 목록, 활성화 토글을 렌더링.
 * 클릭으로 툴 목록 펼치기/접기 가능.
 */
function SkillCard({ skill, onToggle }: SkillCardProps) {
  // 툴 목록 펼침 상태
  const [expanded, setExpanded] = useState(false);

  const isToggleable = skill.status !== "loading" && skill.status !== "error";
  const isActive     = skill.status === "active";

  return (
    <article
      className={`aios-card mb-2 overflow-hidden transition-all duration-200 ${
        isActive ? "border-[#1E1E2E] shadow-aios-card" : "opacity-60 border-[#16161F]"
      }`}
    >
      {/* 카드 헤더 */}
      <div className="px-3 pt-3 pb-2">
        <div className="flex items-start justify-between gap-2">
          {/* 이름 + 버전 */}
          <div className="flex items-center gap-2 min-w-0">
            <div className={`status-dot shrink-0 ${STATUS_DOT_COLOR[skill.status]}`} />
            <div className="min-w-0">
              <div className="flex items-center gap-1.5">
                <span className="text-xs font-semibold text-[#E2E8F0] aios-mono truncate">
                  {skill.name}
                </span>
                <span className="text-[9px] text-[#334155] aios-mono shrink-0">
                  v{skill.version}
                </span>
              </div>
              {/* 상태 텍스트 */}
              <div className="text-[10px] text-[#64748B] mt-0.5">
                {STATUS_LABEL[skill.status]} · 마지막 사용: {relativeTime(skill.lastUsedAt)}
              </div>
            </div>
          </div>

          {/* 토글 버튼 */}
          <button
            onClick={() => onToggle(skill.id)}
            disabled={!isToggleable}
            className={`shrink-0 relative w-8 h-4 rounded-full transition-colors duration-200 ${
              isActive ? "bg-[#6366F1]" : "bg-[#1E1E2E]"
            } ${!isToggleable ? "cursor-not-allowed opacity-50" : "cursor-pointer"}`}
            aria-label={`${skill.name} ${isActive ? "비활성화" : "활성화"}`}
            aria-checked={isActive}
            role="switch"
          >
            <span
              className={`absolute top-0.5 w-3 h-3 rounded-full bg-white shadow transition-transform duration-200 ${
                isActive ? "translate-x-4" : "translate-x-0.5"
              }`}
            />
          </button>
        </div>

        {/* 설명 */}
        <p className="text-[11px] text-[#64748B] mt-2 leading-relaxed line-clamp-2">
          {skill.description}
        </p>

        {/* 툴 수 + 펼치기 버튼 */}
        <button
          onClick={() => setExpanded((p) => !p)}
          className="flex items-center gap-1 mt-2 text-[10px] text-[#475569] hover:text-[#94A3B8] transition-colors"
          aria-expanded={expanded}
        >
          <Wrench size={10} />
          <span>{skill.tools.length}개 도구</span>
          {expanded
            ? <ChevronDown size={10} className="ml-auto" />
            : <ChevronRight size={10} className="ml-auto" />
          }
        </button>
      </div>

      {/* 툴 목록 (확장 시) */}
      {expanded && (
        <div className="border-t border-[#1E1E2E] px-3 py-2 bg-[#0F0F16]">
          {skill.tools.map((tool) => (
            <div key={tool.name} className="flex gap-2 py-1.5 border-b border-[#1A1A28] last:border-0">
              <Zap size={10} className="text-[#6366F1] shrink-0 mt-0.5" />
              <div>
                <div className="text-[10px] font-mono font-medium text-[#A5B4FC]">
                  {tool.name}
                </div>
                <div className="text-[9px] text-[#475569] leading-relaxed mt-0.5">
                  {tool.description}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </article>
  );
}

// ─────────────────────────────────────────────────────────────
//  공개 컴포넌트: SkillList
// ─────────────────────────────────────────────────────────────

/**
 * SkillList
 *
 * 설치된 AIOS 스킬 목록을 좌측 사이드바로 표시.
 * 스킬 활성/비활성 토글, 툴 상세 펼치기/접기를 제공.
 *
 * @example
 * <SkillList />
 */
export function SkillList() {
  const { skills, toggleSkill, activeCount } = useSkills();

  return (
    <aside className="w-[220px] shrink-0 flex flex-col h-full aios-panel border-r">
      {/* 사이드바 헤더 */}
      <div className="flex items-center gap-2 px-3 py-3 border-b border-[#1E1E2E] shrink-0">
        <Package size={14} className="text-[#6366F1]" />
        <span className="text-xs font-semibold text-[#94A3B8] uppercase tracking-widest">
          Skills
        </span>
        {/* 활성 / 전체 뱃지 */}
        <div className="ml-auto flex gap-1">
          <span className="aios-badge bg-[#1a1f3d] text-[#6366F1]">
            {activeCount}
          </span>
          <span className="aios-badge bg-[#1E1E2E] text-[#64748B]">
            {skills.length}
          </span>
        </div>
      </div>

      {/* 스킬 목록 (스크롤 가능) */}
      <div className="flex-1 overflow-y-auto p-2">
        {skills.length === 0 ? (
          <div className="text-center py-8 text-[#334155] text-xs">
            설치된 스킬 없음
          </div>
        ) : (
          skills.map((skill) => (
            <SkillCard
              key={skill.id}
              skill={skill}
              onToggle={toggleSkill}
            />
          ))
        )}
      </div>

      {/* 하단 안내 */}
      <div className="border-t border-[#1E1E2E] px-3 py-2 shrink-0">
        <div className="text-[10px] text-[#334155] leading-relaxed">
          스킬 설치: <span className="aios-mono text-[#475569]">~/.aios/skills/</span>
        </div>
      </div>
    </aside>
  );
}
