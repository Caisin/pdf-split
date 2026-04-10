import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pathsLookSame, pickOutputDir } from "../common/dialog";
import type {
  BatchImageWatermarkProgress,
  BatchImageWatermarkResult,
  InputDirectoryImageListResult,
  MessageTone,
  PreviewImageBytesResult,
} from "../tool-types";

const DEFAULT_WATERMARK_TEXT = "仅限xxx使用,它用或复印无效";
const PREVIEW_DEBOUNCE_MS = 400;

export function BatchImageWatermarkTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState(DEFAULT_WATERMARK_TEXT);
  const [longEdgeFontRatio, setLongEdgeFontRatio] = useState(2.8);
  const [opacity, setOpacity] = useState(18);
  const [rotation, setRotation] = useState(-35);
  const [horizontalSpacingRatio, setHorizontalSpacingRatio] = useState(18);
  const [verticalSpacingRatio, setVerticalSpacingRatio] = useState(12);
  const [previewImageFiles, setPreviewImageFiles] = useState<string[]>([]);
  const [selectedPreviewImage, setSelectedPreviewImage] = useState("");
  const [previewImageUrl, setPreviewImageUrl] = useState("");
  const [previewImageMessage, setPreviewImageMessage] = useState("选择输入目录后，将自动生成真实预览。");
  const [previewBusy, setPreviewBusy] = useState(false);
  const previewObjectUrlRef = useRef("");
  const previewRequestIdRef = useRef(0);
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
    Number.isFinite(longEdgeFontRatio) &&
    longEdgeFontRatio > 0 &&
    longEdgeFontRatio <= 100 &&
    Number.isFinite(opacity) &&
    opacity > 0 &&
    opacity <= 100 &&
    Number.isFinite(rotation) &&
    Number.isFinite(horizontalSpacingRatio) &&
    horizontalSpacingRatio >= 0 &&
    horizontalSpacingRatio <= 100 &&
    Number.isFinite(verticalSpacingRatio) &&
    verticalSpacingRatio >= 0 &&
    verticalSpacingRatio <= 100;

  useEffect(() => {
    if (inputDir === "" || selectedPreviewImage === "") {
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
      setPreviewImageMessage("正在生成真实预览...");
    }

    async function loadPreviewImage() {
      try {
        const result = await invoke<PreviewImageBytesResult>("generate_input_directory_image_preview", {
          payload: {
            inputDir,
            relativePath: selectedPreviewImage,
            watermarkText,
            watermarkLongEdgeFontRatio: longEdgeFontRatio,
            watermarkOpacity: opacity,
            watermarkRotation: rotation,
            watermarkHorizontalSpacingRatio: horizontalSpacingRatio,
            watermarkVerticalSpacingRatio: verticalSpacingRatio,
          },
        });
        if (!active || previewRequestIdRef.current !== requestId) {
          return;
        }

        const nextObjectUrl = URL.createObjectURL(
          new Blob([new Uint8Array(result.bytes)], { type: getImageMimeType(selectedPreviewImage) }),
        );
        if (previewObjectUrlRef.current !== "") {
          URL.revokeObjectURL(previewObjectUrlRef.current);
        }
        previewObjectUrlRef.current = nextObjectUrl;
        setPreviewImageUrl(nextObjectUrl);
        setPreviewBusy(false);
        setPreviewImageMessage(`真实预览：${selectedPreviewImage}`);
      } catch (_error) {
        if (!active || previewRequestIdRef.current !== requestId) {
          return;
        }

        setPreviewBusy(false);
        setPreviewImageMessage("真实预览生成失败，请检查参数或更换图片后重试。");
      }
    }

    return () => {
      active = false;
      window.clearTimeout(previewTimer);
    };
  }, [
    inputDir,
    longEdgeFontRatio,
    opacity,
    rotation,
    selectedPreviewImage,
    horizontalSpacingRatio,
    verticalSpacingRatio,
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
      setSelectedPreviewImage("");
      try {
        const result = await invoke<InputDirectoryImageListResult>("list_input_directory_images", {
          inputDir: selected,
        });
        setPreviewImageFiles(result.files);
        setSelectedPreviewImage(result.files[0] ?? "");
        setPreviewBusy(result.files.length > 0);
        setPreviewImageMessage(
          result.files.length > 0
            ? "正在生成真实预览..."
            : "目录内未找到可预览图片，无法生成真实预览。",
        );
      } catch (_error) {
        setPreviewImageFiles([]);
        setPreviewImageMessage("预览图片列表加载失败，无法生成真实预览。");
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
      unlistenProgress = await listen<BatchImageWatermarkProgress>(
        "batch-image-watermark-progress",
        ({ payload }) => {
          setProgress(payload);
          setMessage(formatBatchImageProgress(payload));
        },
      );
      await yieldToBrowser();

      const result = await invoke<BatchImageWatermarkResult>("add_text_watermark_to_images", {
        payload: {
          inputDir,
          outputDir,
          watermarkText,
          watermarkLongEdgeFontRatio: longEdgeFontRatio,
          watermarkOpacity: opacity,
          watermarkRotation: rotation,
          watermarkHorizontalSpacingRatio: horizontalSpacingRatio,
          watermarkVerticalSpacingRatio: verticalSpacingRatio,
        },
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
          <span>长边字号比例 (%)</span>
          <div className="input-shell">
            <input
              aria-label="长边字号比例"
              type="number"
              min={0.1}
              max={100}
              step={0.1}
              value={longEdgeFontRatio}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setLongEdgeFontRatio(Number.isFinite(nextValue) ? nextValue : 0);
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
          <span>横向间距比例 (%)</span>
          <div className="input-shell">
            <input
              aria-label="图片水印横向间距比例"
              type="number"
              min={0}
              max={100}
              step={0.1}
              value={horizontalSpacingRatio}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setHorizontalSpacingRatio(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>纵向间距比例 (%)</span>
          <div className="input-shell">
            <input
              aria-label="图片水印纵向间距比例"
              type="number"
              min={0}
              max={100}
              step={0.1}
              value={verticalSpacingRatio}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setVerticalSpacingRatio(Number.isFinite(nextValue) ? nextValue : 0);
              }}
            />
          </div>
        </label>
      </div>

      <p className="status-line idle">字号将按图片长边自动计算，并限制在短边的 1% - 50% 之间</p>

      <section className="preview-panel">
        <div className="preview-panel-head">
          <span>参数预览</span>
          <p>{previewImageMessage}</p>
        </div>
        {previewImageFiles.length > 0 && (
          <label className="field">
            <span>预览图片</span>
            <div className="input-shell">
              <select
                aria-label="预览图片"
                value={selectedPreviewImage}
                onChange={(event) => setSelectedPreviewImage(event.currentTarget.value)}
              >
                {previewImageFiles.map((file) => (
                  <option key={file} value={file}>
                    {file}
                  </option>
                ))}
              </select>
            </div>
          </label>
        )}
        <div
          aria-label="图片水印参数预览"
          className={`watermark-preview ${previewImageUrl !== "" ? "has-image" : ""}`}
          role="img"
        >
          {previewBusy && <div className="watermark-preview-updating">更新中</div>}
          {previewImageUrl !== "" && (
            <img
              alt={`真实预览图：${selectedPreviewImage}`}
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

function getImageMimeType(filePath: string) {
  const lowerFilePath = filePath.toLowerCase();
  if (lowerFilePath.endsWith(".png")) {
    return "image/png";
  }
  if (lowerFilePath.endsWith(".webp")) {
    return "image/webp";
  }
  if (lowerFilePath.endsWith(".gif")) {
    return "image/gif";
  }
  if (lowerFilePath.endsWith(".bmp")) {
    return "image/bmp";
  }
  if (lowerFilePath.endsWith(".tif") || lowerFilePath.endsWith(".tiff")) {
    return "image/tiff";
  }

  return "image/jpeg";
}
