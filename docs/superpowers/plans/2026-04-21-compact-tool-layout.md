# Compact Tool Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the split, extract, and batch PDF watermark tool pages use the same dense, space-efficient layout language as the batch image watermark page.

**Architecture:** Reuse the existing dense card and grid classes instead of inventing a new form system. Update the three tool components to group pickers into `picker-grid`, move short controls into existing compact grids, and add only the minimum CSS needed for consistent spacing and responsive fallback.

**Tech Stack:** React, TypeScript, Vitest, existing `App.css` utility classes.

---

### Task 1: Lock the intended dense layout in tests

**Files:**
- Modify: `src/app.test.tsx`

- [ ] **Step 1: Write failing tests for dense layout classes and grouped pickers**
- [ ] **Step 2: Run `pnpm test -- --run src/app.test.tsx` and verify the new expectations fail before implementation**

### Task 2: Make split and extract tools use compact grids

**Files:**
- Modify: `src/components/tools/SplitPdfTool.tsx`
- Modify: `src/components/tools/ExtractImagesTool.tsx`

- [ ] **Step 1: Switch both forms to `tool-card tool-card-dense`**
- [ ] **Step 2: Wrap file/output pickers in `picker-grid`**
- [ ] **Step 3: Keep long content full-width and short controls inside existing compact grid containers**
- [ ] **Step 4: Run the focused frontend tests**

### Task 3: Make batch PDF watermark match the dense form structure

**Files:**
- Modify: `src/components/tools/PdfWatermarkTool.tsx`

- [ ] **Step 1: Switch to dense card styling**
- [ ] **Step 2: Group input/output directory pickers into `picker-grid`**
- [ ] **Step 3: Move short numeric control(s) into a compact field grid while keeping textarea full-width**
- [ ] **Step 4: Run the focused frontend tests**

### Task 4: Add minimal CSS support and verify app-wide layout still builds

**Files:**
- Modify: `src/App.css`

- [ ] **Step 1: Add only the minimum CSS needed for compact grouped layout consistency**
- [ ] **Step 2: Preserve responsive fallback to single-column on narrow widths**
- [ ] **Step 3: Run `pnpm test` and `pnpm build`**
