# Batch Video Watermark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dedicated top-level batch video watermark tab that uses the upstream `Imgs::overlay_slanted_watermark_onto_videos_with_progress(...)` API, shows a first-frame preview, and keeps the UI responsive during long-running work.

**Architecture:** Reuse the existing batch image watermark page structure and Tauri progress-event pattern. Add a dedicated video command path and preview helper in the Tauri layer, map result/progress payloads into frontend types, and keep the preview path thin by extracting the first video frame then applying the same slanted image watermark rendering path used beneath video watermarking.

**Tech Stack:** React, TypeScript, Vitest, Tauri 2, Rust, `kx-image`, ffmpeg/ffprobe-backed upstream video helpers.

---

### Task 1: Lock the new video tool behavior in frontend tests

**Files:**
- Modify: `src/app.test.tsx`

- [ ] **Step 1: Add failing tests for the new top-level tab, default disabled state, preview loading, command invocation, and progress display**
- [ ] **Step 2: Run `pnpm test -- --run src/app.test.tsx` and verify the new expectations fail before implementation**

### Task 2: Add frontend types and tab wiring

**Files:**
- Modify: `src/components/tool-types.ts`
- Modify: `src/components/toolTabs.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Add batch video result/progress/preview types**
- [ ] **Step 2: Add a new `videoWatermark` tab entry**
- [ ] **Step 3: Mount the new tool page in `App.tsx`**
- [ ] **Step 4: Re-run the focused frontend tests**

### Task 3: Implement the new batch video watermark tool UI

**Files:**
- Create: `src/components/tools/BatchVideoWatermarkTool.tsx`
- Modify: `src/App.css` (only if existing shared classes are insufficient)

- [ ] **Step 1: Build a dense tool card that mirrors the batch image watermark structure**
- [ ] **Step 2: Add first-video preview loading with debounce and reuse the existing preview panel style**
- [ ] **Step 3: Add progress-event listening and command submission flow**
- [ ] **Step 4: Re-run the focused frontend tests**

### Task 4: Add Tauri models and commands for batch video watermarking

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add batch video input/result/progress payload models**
- [ ] **Step 2: Add a non-blocking batch video watermark command using `spawn_blocking` and progress events**
- [ ] **Step 3: Add helper logic to list previewable videos and render the first-frame preview bytes**
- [ ] **Step 4: Register the new commands in `lib.rs`**
- [ ] **Step 5: Add/adjust Rust tests for validation, progress forwarding, and preview helper behavior**
- [ ] **Step 6: Run `cd src-tauri && cargo test -q`**

### Task 5: Full verification

**Files:**
- Modify as needed based on failures discovered in verification

- [ ] **Step 1: Run `pnpm test`**
- [ ] **Step 2: Run `pnpm build`**
- [ ] **Step 3: Run `cd src-tauri && cargo test -q`**
- [ ] **Step 4: Fix any regressions and repeat until all checks pass**
