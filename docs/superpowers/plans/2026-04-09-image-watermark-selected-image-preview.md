# Selected Image Preview For Watermark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users choose one image from the selected input directory and see a backend-rendered real watermark preview that auto-refreshes after parameter changes.

**Architecture:** Add one lightweight Tauri command to enumerate previewable images and one real-preview command that uses the existing backend watermark pipeline to render a temporary single-image preview. Extend `BatchImageWatermarkTool` to load the preview list, select one image, debounce parameter changes for 400ms, and display the generated preview bytes as the main preview.

**Tech Stack:** React, TypeScript, Tauri commands, Rust std fs/path, React Testing Library, existing app CSS

---

### Task 1: Lock UI behavior with failing tests

**Files:**
- Modify: `src/app.test.tsx`

- [ ] **Step 1: Write the failing test**
Add tests for loading preview image options after choosing an input directory and for debounced real preview generation.

- [ ] **Step 2: Run test to verify it fails**
Run: `npm test -- src/app.test.tsx`
Expected: FAIL because preview image selection does not exist yet.

- [ ] **Step 3: Write minimal implementation**
Implement the smallest frontend/backend surface needed for the failing tests.

- [ ] **Step 4: Run test to verify it passes**
Run: `npm test -- src/app.test.tsx`
Expected: PASS.

### Task 2: Verify command and app regressions

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/tools/BatchImageWatermarkTool.tsx`
- Modify: `src/components/tool-types.ts`

- [ ] **Step 1: Run frontend tests**
Run: `npm test`
Expected: PASS.

- [ ] **Step 2: Run Rust tests**
Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 3: Run build verification**
Run: `npm run build`
Expected: PASS.
