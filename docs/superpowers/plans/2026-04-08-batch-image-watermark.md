# 批量图片文字水印 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为当前 Tauri 应用新增“批量图片文字水印”工具，支持递归子目录、保留输出结构、平铺斜向重复文字水印和成功/失败汇总。

**Architecture:** React 前端新增一个独立工具页签用于收集目录与水印参数；Tauri command 负责参数校验和结果映射；本地 `kx-pdf` 负责递归遍历、图片渲染、图层叠加与汇总统计。优先复用现有 PDF 水印中的字体解析、文字绘制与旋转逻辑，避免重复造轮子。

**Tech Stack:** React 19、Vite、TypeScript、Vitest、Tauri 2、Rust、`kx-pdf`、`image`、`rusttype`

---

## 文件结构与职责

- 修改：`src/App.tsx`
  - 新增“批量图片水印”页签、表单状态、校验与调用逻辑
- 修改：`src/App.css`
  - 新增批量图片水印表单所需样式复用/微调
- 修改：`src/app.test.tsx`
  - 为新页签和前端校验补充回归测试
- 修改：`src-tauri/Cargo.toml`
  - 将 `kx-pdf` 切到本地 path 依赖以承载新增能力
- 修改：`src-tauri/src/lib.rs`
  - 注册新的 Tauri command
- 修改：`src-tauri/src/commands.rs`
  - 新增命令和参数校验
- 修改：`src-tauri/src/models.rs`
  - 新增批量图片水印结果模型
- 修改：`/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
  - 导出新的图片水印能力
- 新建：`/Volumes/data/code/rust/kx/crates/pdf/src/image_watermark.rs`
  - 实现递归目录图片水印
- 修改：`/Volumes/data/code/rust/kx/crates/pdf/src/watermark.rs`
  - 抽取/复用文字 stamp 与图层渲染逻辑

## Task 1: 先锁定前端交互回归

**Files:**
- Modify: `src/app.test.tsx`

- [ ] 写失败测试，断言新页签和默认禁用状态存在。
- [ ] 运行 `pnpm test -- src/app.test.tsx`，确认因为功能尚未实现而失败。
- [ ] 再写失败测试，覆盖“输入目录和输出目录相同”时按钮禁用。
- [ ] 再次运行相同测试，确认失败原因正确。

## Task 2: 先锁定 Rust 图片批处理行为

**Files:**
- Create: `/Volumes/data/code/rust/kx/crates/pdf/src/image_watermark.rs`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/watermark.rs`

- [ ] 先在新模块里写失败测试：递归处理子目录图片并保留输出结构。
- [ ] 运行 `cargo test -p kx-pdf batch_image_watermark`，确认因接口缺失而失败。
- [ ] 再写失败测试：输入输出目录相同会报错。
- [ ] 再写失败测试：坏图像不会阻塞其它图片输出。
- [ ] 再次运行测试，确认仍为“功能缺失”方向的失败。

## Task 3: 实现 `kx-pdf` 图片水印最小闭环

**Files:**
- Create: `/Volumes/data/code/rust/kx/crates/pdf/src/image_watermark.rs`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/watermark.rs`

- [ ] 新增批量图片水印结果类型与参数类型。
- [ ] 实现目录递归、支持格式识别、输出结构保留。
- [ ] 复用文字渲染和旋转逻辑，支持透明度、角度、横向间距、纵向间距。
- [ ] 实现逐文件失败继续、最终结果汇总。
- [ ] 运行 `cargo test -p kx-pdf image_watermark -- --nocapture`，确认通过。

## Task 4: 接入当前 Tauri 应用命令层

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models.rs`

- [ ] 先写失败测试：新命令拒绝相同输入输出目录。
- [ ] 先写失败测试：新命令拒绝非法透明度与非法间距。
- [ ] 运行 `cargo test`，确认失败。
- [ ] 切换 `kx-pdf` 为本地 path 依赖并实现命令/模型/注册。
- [ ] 重新运行 `cargo test`，确认命令测试通过。

## Task 5: 实现前端页签与表单

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`
- Modify: `src/app.test.tsx`

- [ ] 在现有顶部 Tabs 中新增“批量图片水印”页签。
- [ ] 实现目录选择、参数输入、前端禁用规则和冲突提示。
- [ ] 调用新命令并展示扫描/成功/失败汇总。
- [ ] 运行 `pnpm test -- src/app.test.tsx`，确认前端测试通过。

## Task 6: 完整验证

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`
- Modify: `src/app.test.tsx`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/watermark.rs`
- Create: `/Volumes/data/code/rust/kx/crates/pdf/src/image_watermark.rs`

- [ ] 运行 `pnpm test`。
- [ ] 运行 `pnpm build`。
- [ ] 运行 `cargo test`（`src-tauri`）。
- [ ] 运行 `cargo test -p kx-pdf`（本地 `kx` 工作区）。
- [ ] 如有失败，逐项修复直至通过。
