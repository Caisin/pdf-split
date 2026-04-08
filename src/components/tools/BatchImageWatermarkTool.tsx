import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pathsLookSame, pickOutputDir } from "../common/dialog";
import type {
  BatchImageWatermarkProgress,
  BatchImageWatermarkResult,
  MessageTone,
} from "../tool-types";

const DEFAULT_WATERMARK_TEXT = "仅限xxx使用,它用或复印无效";

export function BatchImageWatermarkTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState(DEFAULT_WATERMARK_TEXT);
  const [fontSize, setFontSize] = useState(28);
  const [opacity, setOpacity] = useState(18);
  const [rotation, setRotation] = useState(-35);
  const [horizontalSpacing, setHorizontalSpacing] = useState(180);
  const [verticalSpacing, setVerticalSpacing] = useState(120);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<BatchImageWatermarkProgress | null>(null);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState<MessageTone>("idle");

  const directoryConflict = inputDir !== "" && outputDir !== "" && pathsLookSame(inputDir, outputDir);
  const canSubmit =
    inputDir !== "" &&
    outputDir !== "" &&
    !directoryConflict &&
    watermarkText.trim() !== "" &&
    Number.isFinite(fontSize) &&
    fontSize > 0 &&
    Number.isFinite(opacity) &&
    opacity > 0 &&
    opacity <= 100 &&
    Number.isFinite(rotation) &&
    Number.isFinite(horizontalSpacing) &&
    horizontalSpacing >= 0 &&
    Number.isFinite(verticalSpacing) &&
    verticalSpacing >= 0;

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
      unlistenProgress = await listen<BatchImageWatermarkProgress>(
        "batch-image-watermark-progress",
        ({ payload }) => {
          setProgress(payload);
          setMessage(formatBatchImageProgress(payload));
        },
      );
      await yieldToBrowser();

      const result = await invoke<BatchImageWatermarkResult>("add_text_watermark_to_images", {
        inputDir,
        outputDir,
        watermarkText,
        watermarkFontSize: fontSize,
        watermarkOpacity: opacity,
        watermarkRotation: rotation,
        watermarkHorizontalSpacing: horizontalSpacing,
        watermarkVerticalSpacing: verticalSpacing,
      });
      setProgress(null);
      setTone("success");
      setMessage(
        `完成：扫描 ${result.scannedFileCount} 张，成功 ${result.successCount} 张，失败 ${result.failureCount} 张，输出目录 ${result.outputDir}`,
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
        <p className="card-kicker">Tool 04</p>
        <h2>批量图片文字水印</h2>
        <p>递归处理输入目录下的图片，保留目录结构输出，不覆盖原文件。</p>
      </div>

      <div className="picker-grid">
        <PickerField
          label="输入目录"
          placeholder="请选择需要批量处理的图片目录"
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

      <div className="field-grid field-grid-compact">
        <label className="field">
          <span>水印字号</span>
          <div className="input-shell">
            <input
              aria-label="图片水印字号"
              type="number"
              min={12}
              max={72}
              step={1}
              value={fontSize}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setFontSize(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>透明度 (%)</span>
          <div className="input-shell">
            <input
              aria-label="图片水印透明度"
              type="number"
              min={1}
              max={100}
              step={1}
              value={opacity}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setOpacity(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>旋转角度</span>
          <div className="input-shell">
            <input
              aria-label="图片水印旋转角度"
              type="number"
              min={-89}
              max={89}
              step={1}
              value={rotation}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setRotation(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>横向间距</span>
          <div className="input-shell">
            <input
              aria-label="图片水印横向间距"
              type="number"
              min={0}
              max={4096}
              step={10}
              value={horizontalSpacing}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setHorizontalSpacing(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>纵向间距</span>
          <div className="input-shell">
            <input
              aria-label="图片水印纵向间距"
              type="number"
              min={0}
              max={4096}
              step={10}
              value={verticalSpacing}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setVerticalSpacing(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>
      </div>

      <p className={`status-line ${directoryConflict ? "error" : "idle"}`}>
        {directoryConflict
          ? "输入目录与输出目录不能相同"
          : "支持递归处理子目录，输出保持原目录结构与原文件名"}
      </p>

      {busy && (
        <div className="progress-stack">
          <progress
            aria-label="图片水印处理进度"
            max={Math.max(progress?.scannedFileCount ?? 1, 1)}
            value={progress?.processedFileCount ?? 0}
          />
          <p className="progress-caption">{message || "处理中..."}</p>
        </div>
      )}

      <button className="submit-button" type="submit" disabled={!canSubmit || busy}>
        {busy ? "处理中..." : "开始批量生成图片水印"}
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

function formatBatchImageProgress(progress: BatchImageWatermarkProgress) {
  const currentFile = progress.currentFile
    ? `当前文件 ${progress.currentFile}`
    : "正在准备文件列表";
  return `处理中：${progress.processedFileCount} / ${progress.scannedFileCount}（成功 ${progress.successCount}，失败 ${progress.failureCount}）${currentFile}`;
}
