# Image Watermark Auto Font Size Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace fixed watermark font size with a fixed internal auto-sizing rule based on image dimensions.

**Architecture:** Remove the front-end font-size control, stop sending font-size through Tauri payloads, and compute the effective font size inside `kx-image` per image using a long-edge base ratio plus short-edge clamps.

**Tech Stack:** React, TypeScript, Tauri, Rust, kx-image, React Testing Library

---

### Task 1: Lock the new UI/API contract with tests

**Files:**
- Modify: `src/app.test.tsx`

- [ ] **Step 1: Write failing tests**
Add assertions that the batch watermark tool no longer renders a font-size input and no longer sends `watermarkFontSize` in preview/batch payloads.

- [ ] **Step 2: Run test to verify it fails**
Run: `npm test -- src/app.test.tsx`
Expected: FAIL.

- [ ] **Step 3: Implement minimal UI/API changes**
Remove the input and update the payloads.

- [ ] **Step 4: Run test to verify it passes**
Run: `npm test -- src/app.test.tsx`
Expected: PASS.

### Task 2: Lock the backend sizing rule

**Files:**
- Modify: `/Volumes/data/code/rust/kx/crates/image/src/watermark.rs`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/image_watermark.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add failing Rust tests**
Add a unit test for the auto font-size calculation and update affected option structs/tests.

- [ ] **Step 2: Implement backend auto sizing**
Apply the long-edge ratio + short-edge clamp rule inside `kx-image` and wire the fixed ratio from the command layer.

- [ ] **Step 3: Run verification**
Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Run: `cargo test -p kx-image --lib --manifest-path /Volumes/data/code/rust/kx/Cargo.toml`
Run: `cargo test -p kx-pdf image_watermark --manifest-path /Volumes/data/code/rust/kx/Cargo.toml`
Expected: PASS.
