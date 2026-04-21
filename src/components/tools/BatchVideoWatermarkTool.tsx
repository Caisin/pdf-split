import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pathsLookSame, pickOutputDir } from "../common/dialog";
import type {
  BatchVideoWatermarkProgress,
  BatchVideoWatermarkResult,
  InputDirectoryVideoListResult,
  MessageTone,
  PreviewImageBytesResult,
} from "../tool-types";

const DEFAULT_WATERMARK_TEXT = "仅限xxx使用,它用或复印无效";
const PREVIEW_DEBOUNCE_MS = 400;

export function BatchVideoWatermarkTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState(DEFAULT_WATERMARK_TEXT);
  const [lineCount, setLineCount] = useState(10);
  const [fullScreen, setFullScreen] = useState(true);
  const [opacity, setOpacity] = useState(0.5);
  const [stripeGapChars, setStripeGapChars] = useState(2);
  const [rowGapLines, setRowGapLines] = useState(3);
  const [previewVideoPath, setPreviewVideoPath] = useState("");
  const [previewImageUrl, setPreviewImageUrl] = useState("");
  const [previewImageMessage, setPreviewImageMessage] = useState("选择输入目录后，将自动提取第一个视频首帧生成真实预览。");
  const [previewBusy, setPreviewBusy] = useState(false);
  const previewObjectUrlRef = useRef("");
  const previewRequestIdRef = useRef(0);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<BatchVideoWatermarkProgress | null>(null);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState<MessageTone>("idle");

  const directoryConflict = inputDir !== "" && outputDir !== "" && pathsLookSame(inputDir, outputDir);
  const canSubmit =
    inputDir !== "" &&
    outputDir !== "" &&
    !directoryConflict &&
    watermarkText.trim() !== "" &&
    Number.isInteger(lineCount) &&
    lineCount > 0 &&
    Number.isFinite(opacity) &&
    opacity >= 0 &&
    opacity <= 1 &&
    Number.isFinite(stripeGapChars) &&
    stripeGapChars >= 0 &&
    Number.isFinite(rowGapLines) &&
    rowGapLines >= 0;

  useEffect(() => {
    if (inputDir === "" || previewVideoPath === "") {
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
      setPreviewImageMessage("正在生成视频首帧真实预览...");
    }

    async function loadPreviewImage() {
      try {
        const result = await invoke<PreviewImageBytesResult>("generate_input_directory_video_preview", {
          payload: {
            inputDir,
            relativePath: previewVideoPath,
            watermarkText,
            watermarkLineCount: lineCount,
            watermarkFullScreen: fullScreen,
            watermarkOpacity: opacity,
            watermarkStripeGapChars: stripeGapChars,
            watermarkRowGapLines: rowGapLines,
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
        setPreviewImageMessage(`真实预览：${previewVideoPath}`);
      } catch (_error) {
        if (!active || previewRequestIdRef.current !== requestId) {
          return;
        }

        setPreviewBusy(false);
        setPreviewImageMessage("视频首帧预览生成失败，请检查参数或更换目录后重试。");
      }
    }

    return () => {
      active = false;
      window.clearTimeout(previewTimer);
    };
  }, [fullScreen, inputDir, lineCount, opacity, previewVideoPath, rowGapLines, stripeGapChars, watermarkText]);

  async function handlePickInputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setInputDir(selected);
      if (previewObjectUrlRef.current !== "") {
        URL.revokeObjectURL(previewObjectUrlRef.current);
        previewObjectUrlRef.current = "";
      }
      setPreviewImageUrl("");
      setPreviewVideoPath("");
      try {
        const result = await invoke<InputDirectoryVideoListResult>("list_input_directory_videos", {
          inputDir: selected,
        });
        const firstVideo = result.files[0] ?? "";
        setPreviewVideoPath(firstVideo);
        setPreviewBusy(firstVideo !== "");
        setPreviewImageMessage(
          firstVideo !== ""
            ? "正在生成视频首帧真实预览..."
            : "目录内未找到可预览视频，无法生成真实预览。",
        );
      } catch (_error) {
        setPreviewImageMessage("预览视频列表加载失败，无法生成真实预览。");
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
      unlistenProgress = await listen<BatchVideoWatermarkProgress>(
        "batch-video-watermark-progress",
        ({ payload }) => {
          setProgress(payload);
          setMessage(formatBatchVideoProgress(payload));
        },
      );
      await yieldToBrowser();

      const result = await invoke<BatchVideoWatermarkResult>("add_slanted_watermark_to_videos", {
        payload: {
          inputDir,
          outputDir,
          watermarkText,
          watermarkLineCount: lineCount,
          watermarkFullScreen: fullScreen,
          watermarkOpacity: opacity,
          watermarkStripeGapChars: stripeGapChars,
          watermarkRowGapLines: rowGapLines,
        },
      });
      setProgress(null);
      setTone("success");
      setMessage(
        `完成：扫描 ${result.scannedFileCount} 个视频，成功 ${result.successCount} 个，新增水印图 ${result.generatedOverlayCount} 张，复用水印图 ${result.reusedOverlayCount} 张，输出目录 ${result.outputDir}`,
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
        <p className="card-kicker">Tool 05</p>
        <h2>批量视频文字水印</h2>
        <p>递归处理输入目录下的视频，复用 slanted watermark 参数并保持目录结构输出。</p>
      </div>

      <div className="picker-grid">
        <PickerField
          label="输入目录"
          placeholder="请选择需要批量处理的视频目录"
          value={inputDir}
          buttonLabel="选择视频输入目录"
          kind="folder"
          onPick={handlePickInputDir}
        />

        <PickerField
          label="输出目录"
          placeholder="请选择输出目录"
          value={outputDir}
          buttonLabel="选择视频输出目录"
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
          <span>水印行数</span>
          <div className="input-shell">
            <input
              aria-label="视频水印行数"
              type="number"
              min={1}
              step={1}
              value={lineCount}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setLineCount(Number.isFinite(nextValue) ? Math.trunc(nextValue) : 0);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>透明度 (0-1)</span>
          <div className="input-shell">
            <input
              aria-label="视频水印透明度"
              type="number"
              min={0}
              max={1}
              step={0.05}
              value={opacity}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setOpacity(Number.isFinite(nextValue) ? nextValue : -1);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>条间距（字符倍数）</span>
          <div className="input-shell">
            <input
              aria-label="视频水印条间距"
              type="number"
              min={0}
              step={0.1}
              value={stripeGapChars}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setStripeGapChars(Number.isFinite(nextValue) ? nextValue : -1);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>行间距（行高倍数）</span>
          <div className="input-shell">
            <input
              aria-label="视频水印行间距"
              type="number"
              min={0}
              step={0.1}
              value={rowGapLines}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setRowGapLines(Number.isFinite(nextValue) ? nextValue : -1);
              }}
            />
          </div>
        </label>
      </div>

      <label className="field field-checkbox">
        <span>铺满画面</span>
        <div className="input-shell">
          <input
            aria-label="视频铺满画面"
            checked={fullScreen}
            type="checkbox"
            onChange={(event) => setFullScreen(event.currentTarget.checked)}
          />
        </div>
      </label>

      <section className="preview-panel">
        <div className="preview-panel-head">
          <span>参数预览</span>
          <p>{previewImageMessage}</p>
        </div>
        <div
          aria-label="视频水印参数预览"
          className={`watermark-preview ${previewImageUrl !== "" ? "has-image" : ""}`}
          role="img"
        >
          {previewBusy && <div className="watermark-preview-updating">更新中</div>}
          {previewImageUrl !== "" && (
            <img
              alt={`真实预览图：${previewVideoPath}`}
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
            aria-label="视频水印处理进度"
            max={Math.max(progress?.scannedFileCount ?? 1, 1)}
            value={progress?.processedFileCount ?? 0}
          />
          <p className="progress-caption">{message || "处理中..."}</p>
        </div>
      )}

      <button className="submit-button" type="submit" disabled={!canSubmit || busy}>
        {busy ? "处理中..." : "开始批量生成视频水印"}
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

function formatBatchVideoProgress(progress: BatchVideoWatermarkProgress) {
  const currentFile = progress.currentFile
    ? `当前文件 ${progress.currentFile}`
    : "正在准备文件列表";
  return `处理中：${progress.processedFileCount} / ${progress.scannedFileCount}（成功 ${progress.successCount}，新增水印图 ${progress.generatedOverlayCount}，复用水印图 ${progress.reusedOverlayCount}）${currentFile}`;
}
