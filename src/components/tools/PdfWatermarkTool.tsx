import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pathsLookSame, pickOutputDir } from "../common/dialog";
import type { BatchPdfWatermarkProgress, BatchPdfWatermarkResult, MessageTone } from "../tool-types";

const DEFAULT_WATERMARK_TEXT = "仅限xxx使用,它用或复印无效";

export function PdfWatermarkTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState(DEFAULT_WATERMARK_TEXT);
  const [watermarkFontSize, setWatermarkFontSize] = useState(28);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<BatchPdfWatermarkProgress | null>(null);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState<MessageTone>("idle");

  const directoryConflict = inputDir !== "" && outputDir !== "" && pathsLookSame(inputDir, outputDir);
  const canSubmit =
    inputDir !== "" &&
    outputDir !== "" &&
    !directoryConflict &&
    watermarkText.trim() !== "" &&
    Number.isFinite(watermarkFontSize) &&
    watermarkFontSize > 0;

  async function handlePickInputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setInputDir(selected);
      setMessage("");
      setTone("idle");
    }
  }

  async function handlePickOutputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setOutputDir(selected);
      setMessage("");
      setTone("idle");
    }
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || busy) {
      return;
    }

    setBusy(true);
    setProgress(null);
    setMessage("处理中...");
    setTone("idle");

    let unlistenProgress: (() => void) | undefined;

    try {
      unlistenProgress = await listen<BatchPdfWatermarkProgress>(
        "batch-pdf-watermark-progress",
        ({ payload }) => {
          setProgress(payload);
          setMessage(formatBatchPdfProgress(payload));
        },
      );
      await yieldToBrowser();

      const result = await invoke<BatchPdfWatermarkResult>("add_text_watermark_to_pdfs", {
        payload: {
          inputDir,
          outputDir,
          watermarkText,
          watermarkFontSize,
        },
      });
      setProgress(null);
      setTone("success");
      setMessage(
        `完成：扫描 ${result.scannedFileCount} 个 PDF，成功 ${result.successCount} 个，失败 ${result.failureCount} 个，输出目录 ${result.outputDir}`,
      );
    } catch (error) {
      setProgress(null);
      setTone("error");
      setMessage(String(error));
    } finally {
      unlistenProgress?.();
      setBusy(false);
    }
  }

  return (
    <form className="tool-card tool-card-dense" onSubmit={handleSubmit}>
      <div className="card-head">
        <p className="card-kicker">Tool 03</p>
        <h2>批量 PDF 文字水印</h2>
        <p>选择输入目录后递归处理所有 PDF，保持目录结构并输出新的水印文件，不覆盖原文件。</p>
      </div>

      <div className="picker-grid">
        <PickerField
          label="输入目录"
          placeholder="请选择包含 PDF 的输入目录"
          value={inputDir}
          buttonLabel="选择输入目录"
          kind="folder"
          onPick={handlePickInputDir}
        />

        <PickerField
          label="输出目录"
          placeholder="请选择输出目录"
          value={outputDir}
          buttonLabel="选择输出目录"
          kind="folder"
          onPick={handlePickOutputDir}
        />
      </div>

      <label className="field">
        <span>水印文字</span>
        <div className="input-shell input-shell-textarea">
          <textarea
            value={watermarkText}
            placeholder={DEFAULT_WATERMARK_TEXT}
            onChange={(event) => setWatermarkText(event.currentTarget.value)}
          />
        </div>
      </label>

      <div className="field-grid">
        <label className="field">
          <span>水印字号</span>
          <div className="input-shell">
            <input
              aria-label="水印字号"
              type="number"
              min={12}
              max={72}
              step={1}
              value={watermarkFontSize}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setWatermarkFontSize(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>
      </div>

      <p className={`status-line ${directoryConflict ? "error" : "idle"}`}>
        {directoryConflict
          ? "输入目录与输出目录不能相同"
          : "输出文件名会自动追加 -watermarked 后缀"}
      </p>

      {busy && (
        <div className="progress-stack">
          <progress
            aria-label="PDF 水印处理进度"
            max={Math.max(progress?.scannedFileCount ?? 1, 1)}
            value={progress?.processedFileCount ?? 0}
          />
          <p className="progress-caption">{message || "处理中..."}</p>
        </div>
      )}

      <button className="submit-button" type="submit" disabled={!canSubmit || busy}>
        {busy ? "处理中..." : "开始批量生成水印 PDF"}
      </button>

      <p className={`status-line ${tone}`}>{message || "等待执行"}</p>
    </form>
  );
}

async function yieldToBrowser() {
  await new Promise<void>((resolve) => {
    if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
      window.requestAnimationFrame(() => resolve());
      return;
    }

    setTimeout(resolve, 0);
  });
}

function formatBatchPdfProgress(progress: BatchPdfWatermarkProgress) {
  const currentFile = progress.currentFile
    ? `当前文件 ${progress.currentFile}`
    : "正在准备文件列表";
  return `处理中：${progress.processedFileCount} / ${progress.scannedFileCount}（成功 ${progress.successCount}，失败 ${progress.failureCount}）${currentFile}`;
}
