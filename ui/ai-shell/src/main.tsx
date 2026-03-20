// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AIOS Contributors
//
// main.tsx — React 18 엔트리포인트.
// StrictMode를 활성화하여 개발 중 부작용을 조기에 감지.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./index.css";

// ── DOM 마운트 ────────────────────────────────────────────────

const rootEl = document.getElementById("root");
if (!rootEl) {
  throw new Error("[AIOS] #root 엘리먼트를 찾을 수 없습니다. index.html을 확인하세요.");
}

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>
);
