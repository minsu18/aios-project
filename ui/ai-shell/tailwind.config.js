// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      // ── AIOS 커스텀 색상 팔레트 ────────────────────────────
      colors: {
        aios: {
          // 배경 계층
          base:    "#0A0A0F", // 최상위 배경
          panel:   "#111118", // 패널 배경
          card:    "#16161F", // 카드 배경
          surface: "#1C1C28", // 서피스 (hover 등)
          border:  "#1E1E2E", // 테두리

          // 텍스트
          text:    "#E2E8F0", // 기본 텍스트
          muted:   "#64748B", // 보조 텍스트
          subtle:  "#94A3B8", // 설명 텍스트

          // 액센트
          primary:  "#6366F1", // 인디고 (주 강조)
          cyan:     "#22D3EE", // 시안 (보조 강조)
          green:    "#10B981", // 성공/정상
          yellow:   "#F59E0B", // 경고
          red:      "#EF4444", // 오류/위험
          purple:   "#A855F7", // AI/추론

          // 특수
          overlay:  "rgba(10,10,15,0.85)",
        },
      },
      // ── 폰트 ───────────────────────────────────────────────
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "ui-monospace", "monospace"],
      },
      // ── 애니메이션 ─────────────────────────────────────────
      animation: {
        "pulse-slow":  "pulse 2.5s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "blink":       "blink 1s step-end infinite",
        "slide-up":    "slideUp 0.2s ease-out",
        "fade-in":     "fadeIn 0.3s ease-out",
        "progress-fill": "progressFill 0.6s ease-out",
      },
      keyframes: {
        blink: {
          "0%, 100%": { opacity: "1" },
          "50%":       { opacity: "0" },
        },
        slideUp: {
          from: { opacity: "0", transform: "translateY(8px)" },
          to:   { opacity: "1", transform: "translateY(0)" },
        },
        fadeIn: {
          from: { opacity: "0" },
          to:   { opacity: "1" },
        },
        progressFill: {
          from: { width: "0%" },
        },
      },
      // ── 박스 쉐도우 ────────────────────────────────────────
      boxShadow: {
        "aios-glow":    "0 0 20px rgba(99,102,241,0.15)",
        "aios-card":    "0 2px 12px rgba(0,0,0,0.4)",
        "aios-inset":   "inset 0 1px 0 rgba(255,255,255,0.05)",
      },
    },
  },
  plugins: [],
};
