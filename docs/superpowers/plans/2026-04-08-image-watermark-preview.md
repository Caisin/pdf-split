# Image Watermark Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a front-end-only live parameter preview to the batch image watermark tool so users can tune watermark settings before running processing.

**Architecture:** Extend the existing `BatchImageWatermarkTool` with a preview section that renders repeated watermark text over a checkerboard background using DOM/CSS only. Keep all Tauri command payloads unchanged and verify the new UI with focused RTL tests before implementation.

**Tech Stack:** React, TypeScript, Vite, React Testing Library, existing app CSS

---

### Task 1: Lock preview requirements with failing tests

**Files:**
- Modify: `src/app.test.tsx`
- Test: `src/app.test.tsx`

- [ ] **Step 1: Write the failing test**

Add tests that switch to the 批量图片水印 tab, assert a preview region is rendered, then change parameters and expect preview content/styles to update.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/app.test.tsx`
Expected: FAIL because preview UI does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement only enough preview markup/state wiring in `src/components/tools/BatchImageWatermarkTool.tsx` and styles in `src/App.css` to satisfy the new tests.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- src/app.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.test.tsx src/components/tools/BatchImageWatermarkTool.tsx src/App.css docs/superpowers/specs/2026-04-08-image-watermark-preview-design.md docs/superpowers/plans/2026-04-08-image-watermark-preview.md
git commit -m "Help users tune image watermark settings before processing"
```

### Task 2: Verify no regressions in existing watermark flow

**Files:**
- Modify: `src/components/tools/BatchImageWatermarkTool.tsx`
- Test: `src/app.test.tsx`

- [ ] **Step 1: Write/assert regression coverage**

Keep the existing submit/progress tests green while the preview is present.

- [ ] **Step 2: Run targeted verification**

Run: `npm test -- src/app.test.tsx`
Expected: PASS.

- [ ] **Step 3: Run broader verification**

Run: `npm test`
Expected: PASS.

- [ ] **Step 4: Optional style sanity check**

Run: `npm run build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.test.tsx src/components/tools/BatchImageWatermarkTool.tsx src/App.css
git commit -m "Keep batch watermark preview aligned with existing flow"
```
