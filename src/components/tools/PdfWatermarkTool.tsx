import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pathsLookSame, pickOutputDir } from "../common/dialog";
import type {
  BatchPdfWatermarkProgress,
  BatchPdfWatermarkResult,
  InputDirectoryPdfListResult,
  MessageTone,
  PreviewImageBytesResult,
} from "../tool-types";

const DEFAULT_WATERMARK_TEXT = "仅限xxx使用,它用或复印无效";
const DEFAULT_WATERMARK_LONG_EDGE_FONT_RATIO = 0.028;
const DEFAULT_WATERMARK_OPACITY = 0.3;
const DEFAULT_WATERMARK_ROTATION_DEGREES = -35;
const DEFAULT_WATERMARK_STRIPE_GAP_CHARS = 2;
const DEFAULT_WATERMARK_ROW_GAP_LINES = 3;
const PREVIEW_DEBOUNCE_MS = 400;

export function PdfWatermarkTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState(DEFAULT_WATERMARK_TEXT);
  const [watermarkLongEdgeFontRatio, setWatermarkLongEdgeFontRatio] = useState(
    DEFAULT_WATERMARK_LONG_EDGE_FONT_RATIO,
  );
  const [watermarkOpacity, setWatermarkOpacity] = useState(DEFAULT_WATERMARK_OPACITY);
  const [watermarkRotationDegrees, setWatermarkRotationDegrees] = useState(
    DEFAULT_WATERMARK_ROTATION_DEGREES,
  );
  const [watermarkStripeGapChars, setWatermarkStripeGapChars] = useState(
    DEFAULT_WATERMARK_STRIPE_GAP_CHARS,
  );
  const [watermarkRowGapLines, setWatermarkRowGapLines] = useState(
    DEFAULT_WATERMARK_ROW_GAP_LINES,
  );
  const [previewPdfPath, setPreviewPdfPath] = useState("");
  const [previewImageUrl, setPreviewImageUrl] = useState("");
  const [previewImageMessage, setPreviewImageMessage] = useState(
    "选择输入目录后，将自动提取第一个 PDF 首页生成真实预览。",
  );
  const [previewBusy, setPreviewBusy] = useState(false);
  const previewObjectUrlRef = useRef("");
  const previewRequestIdRef = useRef(0);
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
    Number.isFinite(watermarkLongEdgeFontRatio) &&
    watermarkLongEdgeFontRatio > 0 &&
    Number.isFinite(watermarkOpacity) &&
    watermarkOpacity >= 0 &&
    watermarkOpacity <= 1 &&
    Number.isFinite(watermarkRotationDegrees) &&
    Number.isFinite(watermarkStripeGapChars) &&
    watermarkStripeGapChars >= 0 &&
    Number.isFinite(watermarkRowGapLines) &&
    watermarkRowGapLines >= 0;

  useEffect(() => {
    if (inputDir === "" || previewPdfPath === "") {
      setPreviewBusy(false);
      return;
    }

    let active = true;
    const requestId = ++previewRequestIdRef.current;
    const previewTimer = window.setTimeout(() => {
      void loadPreviewImage();
    }, PREVIEW_DEBOUNCE_MS);

    setPreviewBusy(true);
    if (previewImageUrl === "") {
      setPreviewImageMessage("正在生成 PDF 首页真实预览...");
    }

    async function loadPreviewImage() {
      try {
        const result = await invoke<PreviewImageBytesResult>("generate_input_directory_pdf_preview", {
          payload: {
            inputDir,
            relativePath: previewPdfPath,
            watermarkText,
            watermarkLongEdgeFontRatio,
            watermarkOpacity,
            watermarkRotationDegrees,
            watermarkStripeGapChars,
            watermarkRowGapLines,
          },
        });
        if (!active || previewRequestIdRef.current !== requestId) {
          return;
        }

        const nextObjectUrl = URL.createObjectURL(
          new Blob([new Uint8Array(result.bytes)], { type: "image/png" }),
        );
        if (previewObjectUrlRef.current !== "") {
          URL.revokeObjectURL(previewObjectUrlRef.current);
        }
        previewObjectUrlRef.current = nextObjectUrl;
        setPreviewImageUrl(nextObjectUrl);
        setPreviewBusy(false);
        setPreviewImageMessage(
          `真实预览：${previewPdfPath}（倾斜角度 ${formatRotationDegrees(watermarkRotationDegrees)}°）`,
        );
      } catch (_error) {
        if (!active || previewRequestIdRef.current !== requestId) {
          return;
        }

        setPreviewBusy(false);
        setPreviewImageMessage("PDF 首页预览生成失败，请检查参数或更换目录后重试。");
      }
    }

    return () => {
      active = false;
      window.clearTimeout(previewTimer);
    };
  }, [
    inputDir,
    previewImageUrl,
    previewPdfPath,
    watermarkLongEdgeFontRatio,
    watermarkOpacity,
    watermarkRotationDegrees,
    watermarkRowGapLines,
    watermarkStripeGapChars,
    watermarkText,
  ]);

  async function handlePickInputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setInputDir(selected);
      if (previewObjectUrlRef.current !== "") {
        URL.revokeObjectURL(previewObjectUrlRef.current);
        previewObjectUrlRef.current = "";
      }
      setPreviewImageUrl("");
      setPreviewPdfPath("");
      try {
        const result = await invoke<InputDirectoryPdfListResult>("list_input_directory_pdfs", {
          inputDir: selected,
        });
        const firstPdf = result.files[0] ?? "";
        setPreviewPdfPath(firstPdf);
        setPreviewBusy(firstPdf !== "");
        setPreviewImageMessage(
          firstPdf !== ""
            ? "正在生成 PDF 首页真实预览..."
            : "目录内未找到可预览 PDF，无法生成真实预览。",
        );
      } catch (_error) {
        setPreviewImageMessage("预览 PDF 列表加载失败，无法生成真实预览。");
      }
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

  useEffect(() => {
    return () => {
      if (previewObjectUrlRef.current !== "") {
        URL.revokeObjectURL(previewObjectUrlRef.current);
      }
    };
  }, []);

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
          watermarkLongEdgeFontRatio,
          watermarkOpacity,
          watermarkRotationDegrees,
          watermarkStripeGapChars,
          watermarkRowGapLines,
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

      <div className="field-grid field-grid-compact">
        <label className="field">
          <span>长边字号比例</span>
          <div className="input-shell">
            <input
              aria-label="PDF 长边字号比例"
              type="number"
              min={0.001}
              step="any"
              value={watermarkLongEdgeFontRatio}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setWatermarkLongEdgeFontRatio(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>透明度 (0-1)</span>
          <div className="input-shell">
            <input
              aria-label="PDF 水印透明度"
              type="number"
              min={0}
              max={1}
              step={0.05}
              value={watermarkOpacity}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setWatermarkOpacity(Number.isFinite(nextValue) ? nextValue : -1);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>倾斜角度（度）</span>
          <div className="input-shell">
            <input
              aria-label="PDF 水印倾斜角度"
              type="number"
              min={-89}
              max={89}
              step={1}
              value={watermarkRotationDegrees}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setWatermarkRotationDegrees(Number.isFinite(nextValue) ? nextValue : Number.NaN);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>条间距（字符倍数）</span>
          <div className="input-shell">
            <input
              aria-label="PDF 水印条间距"
              type="number"
              min={0}
              step={0.1}
              value={watermarkStripeGapChars}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setWatermarkStripeGapChars(Number.isFinite(nextValue) ? nextValue : -1);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>行间距（行高倍数）</span>
          <div className="input-shell">
            <input
              aria-label="PDF 水印行间距"
              type="number"
              min={0}
              step={0.1}
              value={watermarkRowGapLines}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setWatermarkRowGapLines(Number.isFinite(nextValue) ? nextValue : -1);
              }}
            />
          </div>
        </label>
      </div>

      <section className="preview-panel">
        <div className="preview-panel-head">
          <span>参数预览</span>
          <p>{previewImageMessage}</p>
        </div>
        <div
          aria-label="PDF 水印参数预览"
          className={`watermark-preview ${previewImageUrl !== "" ? "has-image" : ""}`}
          role="img"
        >
          {previewBusy && <div className="watermark-preview-updating">更新中</div>}
          {previewImageUrl !== "" && (
            <img
              alt={`真实预览图：${previewPdfPath}`}
              className="watermark-preview-image"
              src={previewImageUrl}
            />
          )}
          {previewImageUrl === "" && !previewBusy && (
            <div className="watermark-preview-placeholder">{previewImageMessage}</div>
          )}
        </div>
      </section>

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

function formatRotationDegrees(rotationDegrees: number) {
  return rotationDegrees.toFixed(1);
}
