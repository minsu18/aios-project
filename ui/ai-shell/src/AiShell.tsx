// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// This file is part of AIOS (AI-Only Operating System).
//
// AiShell — 메인 AI 대화 인터페이스 컴포넌트.
//
// 아키텍처:
//   사용자 입력 → (M0) 규칙 기반 응답 생성 + HAL 명령 시뮬레이션
//   → 메시지 목록 렌더링 → 스트리밍 타이핑 애니메이션
//
// M1에서는 실제 AI Core REST/WebSocket 연동으로 교체 예정.

import {
  AlertCircle,
  ArrowUp,
  CheckCircle,
  ChevronDown,
  Clock,
  Loader2,
  Terminal,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import type { HalCommand, HalCommandStatus, HalCommandType, Message } from "./types";

// ─────────────────────────────────────────────────────────────
//  M0 모의 응답 엔진
//  실제 인텐트 파서(python/intent_engine)와 연동하기 전까지의 스텁.
// ─────────────────────────────────────────────────────────────

interface MockResponse {
  content:     string;
  halCommands: HalCommand[];
}

/** UUID 생성 헬퍼 (crypto API 사용) */
const uid = () => crypto.randomUUID();

/**
 * 사용자 입력에서 M0 모의 HAL 명령 및 응답 텍스트를 생성.
 * 실제 AI Core가 없는 M0 단계에서 UI 동작을 검증하기 위한 스텁.
 *
 * @param text - 사용자가 입력한 메시지
 * @returns 응답 텍스트 + HAL 명령 목록
 */
function buildMockResponse(text: string): MockResponse {
  const lower = text.toLowerCase();

  // ── 메모리 조회 ──────────────────────────────────────────
  if (lower.includes("메모리") || lower.includes("memory") || lower.includes("ram")) {
    return {
      content: "메모리 상태를 조회합니다. HAL QUERY_STATE(memory) 명령을 실행했습니다.\n\n현재 4 GiB 중 약 25% (1 GiB)가 사용 중입니다. 시스템이 정상 동작 범위 내에 있습니다.",
      halCommands: [
        { id: uid(), type: "QUERY_STATE", resource: "memory", params: {}, status: "done" },
      ],
    };
  }

  // ── CPU 조회 ─────────────────────────────────────────────
  if (lower.includes("cpu") || lower.includes("프로세서") || lower.includes("코어")) {
    return {
      content: "CPU 상태를 조회합니다.\n\nCortex-A72 4코어, 현재 총 사용률 약 18%입니다. 로드 에버리지(1m): 0.42로 안정적입니다.",
      halCommands: [
        { id: uid(), type: "QUERY_STATE", resource: "cpu", params: {}, status: "done" },
      ],
    };
  }

  // ── 메모리 할당 ──────────────────────────────────────────
  if (lower.includes("할당") || lower.includes("alloc")) {
    const sizeMatch = text.match(/(\d+)\s*(mb|mib|gb|gib|kb|kib)/i);
    const sizeStr   = sizeMatch ? sizeMatch[0] : "256 MiB";
    return {
      content: `${sizeStr} 메모리 블록 할당을 요청합니다.\n\n보안 검사(CommandSanitizer) → 권한 확인 → 할당 완료.\n핸들 ID: MEM-0x0042`,
      halCommands: [
        { id: uid(), type: "QUERY_STATE",  resource: "memory", params: {},                       status: "done" },
        { id: uid(), type: "ALLOC_MEM",    resource: "memory", params: { size: sizeStr, align: 4096 }, status: "done" },
      ],
    };
  }

  // ── 파일 쓰기 ────────────────────────────────────────────
  if (lower.includes("파일") && (lower.includes("쓰기") || lower.includes("저장") || lower.includes("write"))) {
    return {
      content: "파일 쓰기 요청을 처리합니다.\n\nPath traversal 검사 통과 → 파일 디스크립터 오픈 → 데이터 기록 완료.",
      halCommands: [
        { id: uid(), type: "QUERY_STATE", resource: "storage", params: {},                           status: "done" },
        { id: uid(), type: "OPEN_FILE",   resource: "storage", params: { path: "/tmp/aios.log" },   status: "done" },
        { id: uid(), type: "WRITE_FILE",  resource: "storage", params: { bytes: 128, offset: 0 },  status: "done" },
      ],
    };
  }

  // ── CPU 힌트 ─────────────────────────────────────────────
  if (lower.includes("어피니티") || lower.includes("affinity") || lower.includes("고정") || lower.includes("힌트")) {
    return {
      content: "CPU 어피니티 힌트를 설정합니다.\n\nCore 0-1에 태스크를 고정했습니다. sched_setaffinity(pid=0, mask=0b0011) 완료.",
      halCommands: [
        { id: uid(), type: "CPU_HINT", resource: "cpu", params: { cores: [0, 1], pid: 0 }, status: "done" },
      ],
    };
  }

  // ── 스킬 등록 ────────────────────────────────────────────
  if (lower.includes("스킬") && (lower.includes("등록") || lower.includes("설치"))) {
    return {
      content: "스킬 등록 요청을 처리합니다.\n\nSKILL.md 파싱 → 권한 검증 → 레지스트리 등록 완료.\n스킬이 활성화되었습니다.",
      halCommands: [
        { id: uid(), type: "REGISTER_SKILL", resource: "skill", params: { name: "new-skill", version: "0.1.0" }, status: "done" },
      ],
    };
  }

  // ── 기본 응답 ────────────────────────────────────────────
  return {
    content: `"${text}"에 대한 요청을 수신했습니다.\n\nM0 단계에서는 규칙 기반 인텐트 파서가 동작합니다. 메모리 조회, CPU 상태, 파일 작업, 어피니티 설정 등을 시도해 보세요.`,
    halCommands: [],
  };
}

// ─────────────────────────────────────────────────────────────
//  내부 컴포넌트: HalCommandBadge
// ─────────────────────────────────────────────────────────────

/** HAL 명령 타입별 색상 */
const CMD_COLOR: Record<HalCommandType, string> = {
  QUERY_STATE:    "text-[#22D3EE] bg-[#0c2233]",
  ALLOC_MEM:      "text-[#A855F7] bg-[#1a0d2e]",
  FREE_MEM:       "text-[#94A3B8] bg-[#1a1a2e]",
  CPU_HINT:       "text-[#F59E0B] bg-[#2a1e0a]",
  OPEN_FILE:      "text-[#10B981] bg-[#0a2218]",
  WRITE_FILE:     "text-[#10B981] bg-[#0a2218]",
  REGISTER_SKILL: "text-[#6366F1] bg-[#1a1f3d]",
};

/** HAL 명령 상태 아이콘 */
function StatusIcon({ status }: { status: HalCommandStatus }) {
  switch (status) {
    case "pending":   return <Clock     size={10} className="text-[#F59E0B]" />;
    case "executing": return <Loader2   size={10} className="text-[#6366F1] animate-spin" />;
    case "done":      return <CheckCircle size={10} className="text-[#10B981]" />;
    case "error":     return <AlertCircle size={10} className="text-[#EF4444]" />;
  }
}

/**
 * HAL 명령 한 건을 인라인 배지로 표시.
 * 명령 타입 + 리소스 + 파라미터 요약 + 실행 상태를 표현.
 */
function HalCommandBadge({ cmd }: { cmd: HalCommand }) {
  const colorClass = CMD_COLOR[cmd.type];
  const paramStr   = Object.entries(cmd.params)
    .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
    .join(", ");

  return (
    <div className={`inline-flex items-center gap-1.5 px-2 py-1 rounded text-[10px] font-mono mr-1.5 mb-1.5 ${colorClass}`}>
      <StatusIcon status={cmd.status} />
      <span className="font-semibold">{cmd.type}</span>
      <span className="opacity-60">({cmd.resource})</span>
      {paramStr && (
        <span className="opacity-40 max-w-[120px] truncate">{paramStr}</span>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
//  내부 컴포넌트: ChatMessage
// ─────────────────────────────────────────────────────────────

/** 메시지 발신 시각 포맷 */
function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString("ko-KR", {
    hour:   "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/**
 * 단일 대화 메시지 버블 컴포넌트.
 * user: 우측 정렬, 인디고 배경.
 * assistant: 좌측 정렬, 패널 배경 + HAL 명령 인라인 표시.
 * system: 중앙 정렬, 회색 배경.
 */
function ChatMessage({ msg }: { msg: Message }) {
  if (msg.role === "system") {
    return (
      <div className="flex justify-center my-2 message-appear">
        <div className="aios-badge bg-[#1E1E2E] text-[#475569] text-[10px]">
          {msg.content}
        </div>
      </div>
    );
  }

  const isUser = msg.role === "user";

  return (
    <div className={`flex gap-3 mb-4 message-appear ${isUser ? "flex-row-reverse" : "flex-row"}`}>
      {/* 아바타 */}
      <div className={`shrink-0 w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-bold ${
        isUser
          ? "bg-[#1a1f3d] text-[#6366F1] border border-[#6366F1]/30"
          : "bg-[#1a0d2e] text-[#A855F7] border border-[#A855F7]/30"
      }`}>
        {isUser ? "U" : "AI"}
      </div>

      {/* 말풍선 + HAL 명령 */}
      <div className={`max-w-[75%] ${isUser ? "items-end" : "items-start"} flex flex-col`}>
        {/* HAL 명령 배지 (어시스턴트 메시지에만) */}
        {!isUser && msg.halCommands && msg.halCommands.length > 0 && (
          <div className="flex flex-wrap mb-1.5">
            {msg.halCommands.map((cmd) => (
              <HalCommandBadge key={cmd.id} cmd={cmd} />
            ))}
          </div>
        )}

        {/* 텍스트 버블 */}
        <div className={`rounded-2xl px-4 py-3 text-sm leading-relaxed ${
          isUser
            ? "bg-[#1a1f3d] text-[#E2E8F0] rounded-tr-sm border border-[#6366F1]/20"
            : "bg-[#16161F] text-[#E2E8F0] rounded-tl-sm border border-[#1E1E2E]"
        }`}>
          {/* 스트리밍 중 커서 */}
          {msg.isStreaming ? (
            <span className="aios-mono">{msg.content}<span className="cursor-blink" /></span>
          ) : (
            <span className="whitespace-pre-wrap">{msg.content}</span>
          )}
        </div>

        {/* 타임스탬프 */}
        <div className="text-[9px] text-[#334155] mt-1 aios-mono px-1">
          {formatTime(msg.timestamp)}
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
//  공개 컴포넌트: AiShell
// ─────────────────────────────────────────────────────────────

/** 시스템 부팅 메시지 (M0 기준) */
const BOOT_MESSAGES: Message[] = [
  {
    id:        uid(),
    role:      "system",
    content:   "AIOS M0 Shell — 세션 시작",
    timestamp: Date.now() - 2000,
  },
  {
    id:        uid(),
    role:      "assistant",
    content:   "안녕하세요. AIOS AI Shell입니다.\n\n메모리 조회, CPU 상태, 파일 작업, 어피니티 설정 등을 자연어로 요청해 보세요. HAL 명령이 실시간으로 표시됩니다.",
    timestamp: Date.now() - 1000,
    halCommands: [],
  },
];

/**
 * AiShell
 *
 * 메인 AI 대화 인터페이스 컴포넌트.
 * 사용자 입력을 받아 M0 규칙 기반 응답 및 HAL 명령을 시뮬레이션하고,
 * 스트리밍 타이핑 효과로 응답을 표시한다.
 *
 * @example
 * <AiShell />
 */
export function AiShell() {
  const [messages,    setMessages]  = useState<Message[]>(BOOT_MESSAGES);
  const [inputText,   setInputText] = useState("");
  const [isThinking,  setIsThinking] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef       = useRef<HTMLTextAreaElement>(null);

  // ── 자동 스크롤 ───────────────────────────────────────────
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // ── 메시지 전송 ───────────────────────────────────────────

  /**
   * 사용자 메시지를 추가하고, M0 모의 응답을 생성해 스트리밍한다.
   * 스트리밍: 어시스턴트 메시지를 isStreaming=true로 먼저 추가한 뒤
   * setTimeout으로 isStreaming=false로 교체 (500ms 지연).
   */
  const handleSend = useCallback(async () => {
    const text = inputText.trim();
    if (!text || isThinking) return;

    // 1. 사용자 메시지 추가
    const userMsg: Message = {
      id:        uid(),
      role:      "user",
      content:   text,
      timestamp: Date.now(),
    };
    setMessages((prev) => [...prev, userMsg]);
    setInputText("");
    setIsThinking(true);

    // 2. M0 응답 생성 (300ms ~ 800ms 랜덤 딜레이로 AI 추론 흉내)
    const delay = 300 + Math.random() * 500;
    await new Promise((r) => setTimeout(r, delay));

    const { content, halCommands } = buildMockResponse(text);
    const assistantId = uid();

    // 3. 스트리밍 상태로 어시스턴트 메시지 추가
    const streamingMsg: Message = {
      id:          assistantId,
      role:        "assistant",
      content,
      timestamp:   Date.now(),
      halCommands,
      isStreaming: true,
    };
    setMessages((prev) => [...prev, streamingMsg]);
    setIsThinking(false);

    // 4. 500ms 후 스트리밍 완료 처리
    setTimeout(() => {
      setMessages((prev) =>
        prev.map((m) =>
          m.id === assistantId ? { ...m, isStreaming: false } : m
        )
      );
    }, 500);
  }, [inputText, isThinking]);

  // ── 키보드 단축키 ─────────────────────────────────────────
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Enter: 전송 / Shift+Enter: 줄바꿈
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void handleSend();
      }
    },
    [handleSend]
  );

  // ── 스크롤 맨 아래로 버튼 표시 여부 ────────────────────────
  const containerRef = useRef<HTMLDivElement>(null);
  const [showScrollBtn, setShowScrollBtn] = useState(false);

  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    setShowScrollBtn(distFromBottom > 200);
  }, []);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  // ─────────────────────────────────────────────────────────
  return (
    <section className="flex-1 flex flex-col min-w-0 h-full">
      {/* 대화 영역 */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto px-6 py-4 relative"
      >
        {messages.map((msg) => (
          <ChatMessage key={msg.id} msg={msg} />
        ))}

        {/* 생각 중 인디케이터 */}
        {isThinking && (
          <div className="flex items-center gap-2 mb-4 message-appear">
            <div className="w-7 h-7 rounded-full bg-[#1a0d2e] border border-[#A855F7]/30 flex items-center justify-center text-[10px] font-bold text-[#A855F7]">
              AI
            </div>
            <div className="bg-[#16161F] border border-[#1E1E2E] rounded-2xl rounded-tl-sm px-4 py-3">
              <div className="flex gap-1 items-center">
                <div className="w-1.5 h-1.5 rounded-full bg-[#6366F1] animate-bounce [animation-delay:0ms]" />
                <div className="w-1.5 h-1.5 rounded-full bg-[#6366F1] animate-bounce [animation-delay:150ms]" />
                <div className="w-1.5 h-1.5 rounded-full bg-[#6366F1] animate-bounce [animation-delay:300ms]" />
              </div>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />

        {/* 스크롤 아래로 버튼 */}
        {showScrollBtn && (
          <button
            onClick={scrollToBottom}
            className="fixed bottom-24 right-[290px] p-2 rounded-full bg-[#16161F] border border-[#1E1E2E] text-[#64748B] hover:text-[#E2E8F0] hover:border-[#6366F1] transition-all shadow-aios-card"
            aria-label="맨 아래로 스크롤"
          >
            <ChevronDown size={14} />
          </button>
        )}
      </div>

      {/* 입력 영역 */}
      <div className="border-t border-[#1E1E2E] px-4 py-3 bg-[#0D0D14] shrink-0">
        {/* 입력 박스 */}
        <div className="flex gap-3 items-end max-w-4xl mx-auto">
          {/* 프롬프트 표시 */}
          <div className="shrink-0 flex items-center gap-1.5 pb-2.5">
            <Terminal size={13} className="text-[#6366F1]" />
            <span className="text-[10px] text-[#334155] aios-mono">AIOS&gt;</span>
          </div>

          {/* 텍스트 영역 */}
          <textarea
            ref={inputRef}
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="자연어로 명령을 입력하세요 (예: 메모리 상태 확인해줘)"
            rows={1}
            className="flex-1 resize-none bg-[#16161F] border border-[#1E1E2E] rounded-xl px-4 py-2.5
                       text-sm text-[#E2E8F0] placeholder:text-[#334155] aios-mono
                       focus:outline-none focus:border-[#6366F1]/50 focus:bg-[#1a1f3d]/20
                       transition-colors min-h-[42px] max-h-[160px] leading-relaxed"
            style={{ fieldSizing: "content" } as React.CSSProperties}
            disabled={isThinking}
            aria-label="AI 명령 입력"
          />

          {/* 전송 버튼 */}
          <button
            onClick={() => void handleSend()}
            disabled={!inputText.trim() || isThinking}
            className="shrink-0 mb-0.5 w-9 h-9 rounded-xl flex items-center justify-center
                       bg-[#6366F1] text-white transition-all
                       hover:bg-[#4F46E5] active:scale-95
                       disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-[#6366F1]"
            aria-label="전송"
          >
            {isThinking
              ? <Loader2 size={15} className="animate-spin" />
              : <ArrowUp  size={15} />
            }
          </button>
        </div>

        {/* 하단 힌트 */}
        <div className="text-center mt-1.5">
          <span className="text-[9px] text-[#1E2A3A]">
            Enter 전송 · Shift+Enter 줄바꿈 · M0 규칙 기반 응답
          </span>
        </div>
      </div>
    </section>
  );
}
