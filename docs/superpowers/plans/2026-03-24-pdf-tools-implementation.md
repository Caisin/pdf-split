# PDF 工具实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为当前 Tauri 应用实现“PDF 按页导出图片”和“PDF 文字水印”两个可用功能，其中当前项目只负责编排与调用，本地 `kx-pdf` 负责真实 PDF 处理。

**Architecture:** 前端页面替换为两个独立功能卡片，使用 Tauri 对话框选择 PDF 和输出目录，通过 Tauri command 调用 Rust。当前项目 Rust 层保持轻量，只做参数校验、调用 `kx-pdf`、结果映射；`kx-pdf` 新增按页转图片与文字水印能力并提供可测试接口。

**Tech Stack:** React 19、Vite、TypeScript、Tauri 2、Rust、`tauri-plugin-dialog`、本地依赖 `kx-pdf`

---

## 文件结构与职责

### 当前项目 `pdf-split`

- 修改：`/Volumes/data/code/tauri/pdf-split/package.json`
  - 增加前端测试命令与需要的测试依赖
- 修改：`/Volumes/data/code/tauri/pdf-split/src-tauri/Cargo.toml`
  - 增加 Tauri 对话框插件与命令层可能需要的依赖
- 修改：`/Volumes/data/code/tauri/pdf-split/src-tauri/src/lib.rs`
  - 注册对话框插件和新命令
- 新建：`/Volumes/data/code/tauri/pdf-split/src-tauri/src/commands.rs`
  - 定义 `split_pdf_to_images`、`add_text_watermark`
- 新建：`/Volumes/data/code/tauri/pdf-split/src-tauri/src/models.rs`
  - 定义命令入参与返回结构
- 修改：`/Volumes/data/code/tauri/pdf-split/src/App.tsx`
  - 实现两个功能卡片与调用逻辑
- 修改：`/Volumes/data/code/tauri/pdf-split/src/App.css`
  - 实现工具页样式
- 新建：`/Volumes/data/code/tauri/pdf-split/src/app.test.tsx`
  - 前端最小行为测试
- 新建：`/Volumes/data/code/tauri/pdf-split/src/test/setup.ts`
  - 测试初始化
- 修改：`/Volumes/data/code/tauri/pdf-split/vite.config.ts`
  - 接入 Vitest 配置

### 本地依赖 `kx-pdf`

- 修改：`/Volumes/data/code/rust/kx/crates/pdf/Cargo.toml`
  - 增加实现两项能力需要的依赖
- 修改：`/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
  - 导出新模块与公共接口
- 新建：`/Volumes/data/code/rust/kx/crates/pdf/src/page_render.rs`
  - 实现 PDF 按页导出图片
- 新建：`/Volumes/data/code/rust/kx/crates/pdf/src/watermark.rs`
  - 实现 PDF 文字水印

说明：

- `/Volumes/data/code/rust/kx/crates/pdf` 不在当前工作目录内。执行该部分计划时，如果沙箱不允许跨目录写入，需要先申请提权后再编辑。

## Task 1：搭建前端测试与文件选择基础设施

**Files:**
- Modify: `/Volumes/data/code/tauri/pdf-split/package.json`
- Modify: `/Volumes/data/code/tauri/pdf-split/vite.config.ts`
- Modify: `/Volumes/data/code/tauri/pdf-split/src-tauri/Cargo.toml`
- Modify: `/Volumes/data/code/tauri/pdf-split/src-tauri/src/lib.rs`
- Create: `/Volumes/data/code/tauri/pdf-split/src/test/setup.ts`

- [ ] **Step 1: 为前端页面行为写一个失败测试**

```tsx
import { render, screen } from "@testing-library/react";
import App from "./App";

test("renders both PDF tool sections", () => {
  render(<App />);
  expect(screen.getByText("PDF 转图片")).toBeInTheDocument();
  expect(screen.getByText("PDF 文字水印")).toBeInTheDocument();
});
```

- [ ] **Step 2: 运行测试，确认它先失败**

Run: `npm test -- src/app.test.tsx`

Expected:
- 失败，原因是测试命令或测试依赖尚未配置完成

- [ ] **Step 3: 最小化接入测试基础设施**

代码目标：

- 在 `package.json` 添加：

```json
{
  "scripts": {
    "test": "vitest run"
  }
}
```

- 增加最小测试依赖：
  - `vitest`
  - `@testing-library/react`
  - `@testing-library/jest-dom`
  - `jsdom`

- 在 `vite.config.ts` 添加：

```ts
test: {
  environment: "jsdom",
  setupFiles: "./src/test/setup.ts",
}
```

- 在 `src/test/setup.ts` 添加：

```ts
import "@testing-library/jest-dom";
```

- 在 `src-tauri/Cargo.toml` 和 `src-tauri/src/lib.rs` 接入 `tauri-plugin-dialog`

- [ ] **Step 4: 重新运行测试，确认基础设施通过**

Run: `npm test -- src/app.test.tsx`

Expected:
- 测试执行成功
- 当前断言仍可能失败，但测试框架应正常工作

- [ ] **Step 5: 提交这一小步**

```bash
git add package.json vite.config.ts src/test/setup.ts src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "test: add frontend test setup and dialog plugin"
```

## Task 2：为 `kx-pdf` 增加按页导出图片接口

**Files:**
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/Cargo.toml`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
- Create: `/Volumes/data/code/rust/kx/crates/pdf/src/page_render.rs`

- [ ] **Step 1: 先写一个失败的 Rust 测试，描述图片导出接口**

```rust
#[test]
fn export_pages_as_png_creates_one_file_per_page() -> anyhow::Result<()> {
    let input = "tests/fixtures/two-pages.pdf";
    let output_dir = tempfile::tempdir()?;

    let result = Pdfs::render_pages_to_images(input, output_dir.path(), "png")?;

    assert_eq!(result.page_count, 2);
    assert_eq!(result.generated_files.len(), 2);
    assert!(result.generated_files[0].ends_with("p001.png"));
    Ok(())
}
```

- [ ] **Step 2: 运行该测试并确认它因接口不存在而失败**

Run: `cargo test -p kx-pdf export_pages_as_png_creates_one_file_per_page`

Expected:
- FAIL
- 报错为方法或类型不存在，而不是测试写错

- [ ] **Step 3: 用最小实现让测试通过**

实现要求：

- 在 `src/page_render.rs` 提供公共接口：

```rust
pub struct RenderImagesResult {
    pub page_count: usize,
    pub generated_files: Vec<String>,
}

impl Pdfs {
    pub fn render_pages_to_images(
        input_path: &str,
        output_dir: &std::path::Path,
        image_format: &str,
    ) -> anyhow::Result<RenderImagesResult> {
        todo!()
    }
}
```

- 输出命名格式：
  - `原文件名-p001.png`
  - `原文件名-p001.jpg`

- 非法格式直接返回错误
- 如目标文件已存在，直接返回错误

- [ ] **Step 4: 重新运行测试，确认通过**

Run: `cargo test -p kx-pdf export_pages_as_png_creates_one_file_per_page`

Expected:
- PASS

- [ ] **Step 5: 增加一个失败测试，覆盖 JPG 与同名冲突**

```rust
#[test]
fn export_pages_as_jpg_rejects_existing_target() -> anyhow::Result<()> {
    // 准备同名目标文件
    // 调用接口
    // 断言返回错误信息包含 exists / already exists
    Ok(())
}
```

- [ ] **Step 6: 运行测试确认失败**

Run: `cargo test -p kx-pdf export_pages_as_jpg_rejects_existing_target`

Expected:
- FAIL

- [ ] **Step 7: 最小化补齐实现**

实现要求：

- 支持 `jpg`
- 同名文件冲突报错
- 返回稳定、可上层映射的错误

- [ ] **Step 8: 运行相关测试确认通过**

Run: `cargo test -p kx-pdf page_render -- --nocapture`

Expected:
- 相关按页导出测试全部通过

- [ ] **Step 9: 提交这一小步**

```bash
git -C /Volumes/data/code/rust/kx add crates/pdf/Cargo.toml crates/pdf/src/lib.rs crates/pdf/src/page_render.rs
git -C /Volumes/data/code/rust/kx commit -m "feat: add PDF page image rendering API"
```

## Task 3：为 `kx-pdf` 增加文字水印接口

**Files:**
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/Cargo.toml`
- Modify: `/Volumes/data/code/rust/kx/crates/pdf/src/lib.rs`
- Create: `/Volumes/data/code/rust/kx/crates/pdf/src/watermark.rs`

- [ ] **Step 1: 先写一个失败测试，描述水印输出行为**

```rust
#[test]
fn add_text_watermark_creates_new_pdf() -> anyhow::Result<()> {
    let input = "tests/fixtures/simple.pdf";
    let output_dir = tempfile::tempdir()?;

    let output = Pdfs::add_text_watermark(input, output_dir.path(), "CONFIDENTIAL")?;

    assert!(output.ends_with("-watermarked.pdf"));
    assert!(std::path::Path::new(&output).exists());
    Ok(())
}
```

- [ ] **Step 2: 运行测试并确认它先失败**

Run: `cargo test -p kx-pdf add_text_watermark_creates_new_pdf`

Expected:
- FAIL

- [ ] **Step 3: 用最小实现让测试通过**

实现要求：

- 新建 `src/watermark.rs`
- 暴露接口：

```rust
impl Pdfs {
    pub fn add_text_watermark(
        input_path: &str,
        output_dir: &std::path::Path,
        watermark_text: &str,
    ) -> anyhow::Result<String> {
        todo!()
    }
}
```

- 输入文本为空直接报错
- 默认输出名：`原文件名-watermarked.pdf`
- 不覆盖原始文件
- 目标文件已存在时报错

- [ ] **Step 4: 重新运行测试，确认通过**

Run: `cargo test -p kx-pdf add_text_watermark_creates_new_pdf`

Expected:
- PASS

- [ ] **Step 5: 再补一个失败测试，覆盖空水印文本**

```rust
#[test]
fn add_text_watermark_rejects_empty_text() {
    let err = Pdfs::add_text_watermark("tests/fixtures/simple.pdf", tempdir.path(), "");
    assert!(err.is_err());
}
```

- [ ] **Step 6: 运行测试确认失败**

Run: `cargo test -p kx-pdf add_text_watermark_rejects_empty_text`

Expected:
- FAIL

- [ ] **Step 7: 最小化补齐实现**

实现要求：

- 空文本报错
- 错误信息稳定

- [ ] **Step 8: 运行相关测试确认通过**

Run: `cargo test -p kx-pdf watermark -- --nocapture`

Expected:
- 相关水印测试全部通过

- [ ] **Step 9: 提交这一小步**

```bash
git -C /Volumes/data/code/rust/kx add crates/pdf/Cargo.toml crates/pdf/src/lib.rs crates/pdf/src/watermark.rs
git -C /Volumes/data/code/rust/kx commit -m "feat: add PDF text watermark API"
```

## Task 4：在当前 Tauri 项目实现命令层与结果模型

**Files:**
- Create: `/Volumes/data/code/tauri/pdf-split/src-tauri/src/models.rs`
- Create: `/Volumes/data/code/tauri/pdf-split/src-tauri/src/commands.rs`
- Modify: `/Volumes/data/code/tauri/pdf-split/src-tauri/src/lib.rs`

- [ ] **Step 1: 先写一个失败的命令层测试**

```rust
#[test]
fn split_command_rejects_empty_input_path() {
    let err = split_pdf_to_images("".into(), "/tmp".into(), "png".into()).unwrap_err();
    assert!(err.contains("PDF 文件"));
}
```

- [ ] **Step 2: 运行测试并确认它先失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml split_command_rejects_empty_input_path`

Expected:
- FAIL

- [ ] **Step 3: 最小化实现命令层**

实现要求：

- 将命令从 `src-tauri/src/lib.rs` 拆到 `commands.rs`
- 在 `models.rs` 定义：

```rust
#[derive(serde::Serialize)]
pub struct SplitPdfResult {
    pub page_count: usize,
    pub generated_file_count: usize,
    pub output_dir: String,
}

#[derive(serde::Serialize)]
pub struct WatermarkPdfResult {
    pub output_pdf_path: String,
}
```

- 在 `commands.rs` 定义：
  - `split_pdf_to_images(input_path, output_dir, image_format)`
  - `add_text_watermark(input_path, output_dir, watermark_text)`

- 命令层只做：
  - 空值校验
  - 调用 `kx-pdf`
  - 映射结果与错误

- [ ] **Step 4: 重新运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml split_command_rejects_empty_input_path`

Expected:
- PASS

- [ ] **Step 5: 补一个失败测试，覆盖空水印文本**

```rust
#[test]
fn watermark_command_rejects_empty_text() {
    let err = add_text_watermark("a.pdf".into(), "/tmp".into(), "".into()).unwrap_err();
    assert!(err.contains("水印文字"));
}
```

- [ ] **Step 6: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml watermark_command_rejects_empty_text`

Expected:
- FAIL

- [ ] **Step 7: 最小化补齐实现**

实现要求：

- 错误信息保持用户可读
- 注册两个命令到 `invoke_handler!`

- [ ] **Step 8: 运行命令层相关测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands -- --nocapture`

Expected:
- 命令层测试通过

- [ ] **Step 9: 提交这一小步**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands.rs src-tauri/src/models.rs src-tauri/Cargo.toml
git commit -m "feat: add Tauri PDF command layer"
```

## Task 5：实现前端页面与命令调用

**Files:**
- Modify: `/Volumes/data/code/tauri/pdf-split/src/App.tsx`
- Modify: `/Volumes/data/code/tauri/pdf-split/src/App.css`
- Create: `/Volumes/data/code/tauri/pdf-split/src/app.test.tsx`

- [ ] **Step 1: 先写一个失败测试，描述页面最小交互**

```tsx
test("disables image export submit until required fields are filled", () => {
  render(<App />);
  expect(screen.getByRole("button", { name: "开始导出图片" })).toBeDisabled();
});
```

- [ ] **Step 2: 运行测试并确认它先失败**

Run: `npm test -- src/app.test.tsx`

Expected:
- FAIL

- [ ] **Step 3: 最小化实现工具页**

实现要求：

- 替换默认欢迎页
- 页面展示两个卡片：
  - `PDF 转图片`
  - `PDF 文字水印`
- 接入文件和目录选择
- 接入 `invoke` 调用对应 Tauri 命令
- 每个卡片有独立 loading / success / error 状态
- 导出图片卡片支持 `PNG/JPG` 选择
- 水印卡片支持文字输入

- [ ] **Step 4: 重新运行测试，确认通过**

Run: `npm test -- src/app.test.tsx`

Expected:
- PASS

- [ ] **Step 5: 补一个失败测试，覆盖水印按钮禁用逻辑**

```tsx
test("disables watermark submit when watermark text is empty", () => {
  render(<App />);
  expect(screen.getByRole("button", { name: "开始生成水印 PDF" })).toBeDisabled();
});
```

- [ ] **Step 6: 运行测试确认失败**

Run: `npm test -- src/app.test.tsx`

Expected:
- FAIL

- [ ] **Step 7: 最小化补齐实现**

实现要求：

- 两个按钮在缺少必填项时禁用
- 成功后展示结果摘要
- 失败后展示错误提示

- [ ] **Step 8: 运行前端测试确认通过**

Run: `npm test -- src/app.test.tsx`

Expected:
- PASS

- [ ] **Step 9: 提交这一小步**

```bash
git add src/App.tsx src/App.css src/app.test.tsx
git commit -m "feat: build PDF tools page"
```

## Task 6：集成验证与最终收口

**Files:**
- Modify: `/Volumes/data/code/tauri/pdf-split/README.md`
  - 补充运行方式和功能说明（如果需要）

- [ ] **Step 1: 运行前端测试**

Run: `npm test`

Expected:
- PASS

- [ ] **Step 2: 运行 Rust 命令层测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS

- [ ] **Step 3: 运行 `kx-pdf` 测试**

Run: `cargo test -p kx-pdf`

Expected:
- PASS

- [ ] **Step 4: 运行当前项目构建校验**

Run: `npm run build`

Expected:
- PASS

- [ ] **Step 5: 运行 Tauri 构建校验**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS

- [ ] **Step 6: 手动验证两个功能**

检查项：

- 选择 PDF 和输出目录流程正常
- `PNG/JPG` 图片导出可执行
- 文字水印可执行
- 错误文案可读
- 原始 PDF 未被覆盖

- [ ] **Step 7: 提交最终整合结果**

```bash
git add README.md package.json vite.config.ts src src-tauri docs/superpowers/plans/2026-03-24-pdf-tools-implementation.md
git commit -m "feat: implement PDF image export and watermark tools"
```
