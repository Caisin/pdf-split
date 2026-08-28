import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pickOutputDir } from "../common/dialog";
import type {
  MessageTone,
  SeriesDownloadListSummary,
  SeriesDownloadProgress,
  SeriesDownloadResult,
} from "../tool-types";

const DEFAULT_CONCURRENT_DOWNLOADS = 5;
const MAX_CONCURRENT_DOWNLOADS = 20;

export function SeriesDownloadTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [concurrentDownloads, setConcurrentDownloads] = useState(
    DEFAULT_CONCURRENT_DOWNLOADS,
  );
  const [summary, setSummary] = useState<SeriesDownloadListSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [inspecting, setInspecting] = useState(false);
  const [progress, setProgress] = useState<SeriesDownloadProgress | null>(null);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState<MessageTone>("idle");

  const concurrencyIsValid =
    Number.isInteger(concurrentDownloads) &&
    concurrentDownloads >= 1 &&
    concurrentDownloads <= MAX_CONCURRENT_DOWNLOADS;
  const canSubmit =
    inputDir !== "" &&
    outputDir !== "" &&
    summary !== null &&
    concurrencyIsValid &&
    !inspecting;

  async function handlePickInputDir() {
    const selected = await pickOutputDir();
    if (!selected) {
      return;
    }

    setInspecting(true);
    setInputDir(selected);
    setSummary(null);
    setMessage("正在读取目录中的 TXT 下载清单...");
    setTone("idle");
    try {
      const nextSummary = await invoke<SeriesDownloadListSummary>(
        "inspect_series_download_directory",
        { inputDir: selected },
      );
      setSummary(nextSummary);
      setTone("success");
      setMessage(formatSummary(nextSummary));
    } catch (error) {
      setInputDir("");
      setTone("error");
      setMessage(String(error));
    } finally {
      setInspecting(false);
    }
  }

  async function handlePickOutputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setOutputDir(selected);
    }
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || busy) {
      return;
    }

    setBusy(true);
    setProgress(null);
    setMessage("正在准备下载...");
    setTone("idle");
    let unlistenProgress: (() => void) | undefined;

    try {
      unlistenProgress = await listen<SeriesDownloadProgress>(
        "series-download-progress",
        ({ payload }) => {
          setProgress(payload);
          setMessage(formatDownloadProgress(payload));
        },
      );
      await yieldToBrowser();

      const result = await invoke<SeriesDownloadResult>("download_series_videos", {
        payload: {
          inputDir,
          outputDir,
          concurrentDownloads,
        },
      });
      setProgress(null);
      if (result.failureCount > 0) {
        setTone("error");
        const firstFailure = result.failedItems[0] ? `；${result.failedItems[0]}` : "";
        setMessage(
          `下载完成，但有失败：成功 ${result.successCount}，失败 ${result.failureCount}，跳过 ${result.skippedCount}${firstFailure}`,
        );
      } else {
        setTone("success");
        setMessage(
          `下载完成：成功 ${result.successCount}，跳过 ${result.skippedCount}，已保存到 ${result.outputDir}`,
        );
      }
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
        <p className="card-kicker">Tool 07</p>
        <h2>剧集下载</h2>
        <p>解析输入目录下的所有 TXT 清单，视频按“剧名/集数.mp4”保存到下载目录。</p>
      </div>

      <div className="picker-grid">
        <PickerField
          label="TXT 清单目录"
          placeholder="请选择包含 TXT 下载清单的目录"
          value={inputDir}
          buttonLabel="选择 TXT 清单目录"
          kind="folder"
          onPick={handlePickInputDir}
        />

        <PickerField
          label="下载目录"
          placeholder="请选择视频保存目录"
          value={outputDir}
          buttonLabel="选择剧集下载目录"
          kind="folder"
          onPick={handlePickOutputDir}
        />
      </div>

      <div className="field-grid">
        <label className="field">
          <span>同时下载数</span>
          <div className="input-shell">
            <input
              aria-label="同时下载数"
              type="number"
              min={1}
              max={MAX_CONCURRENT_DOWNLOADS}
              step={1}
              value={concurrentDownloads}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setConcurrentDownloads(Number.isFinite(nextValue) ? Math.trunc(nextValue) : 0);
              }}
            />
          </div>
        </label>

        <div className="download-summary" aria-live="polite">
          <span>清单内容</span>
          <strong>
            {summary
              ? `${summary.fileCount} 个 TXT · ${summary.series.length} 部剧 · ${summary.episodeCount} 集`
              : "等待导入"}
          </strong>
        </div>
      </div>

      {summary && (
        <p className="series-list" aria-label="已导入剧集">
          {summary.series
            .map((series) => `${series.name}（${series.episodeCount} 集）`)
            .join("、")}
        </p>
      )}

      {busy && (
        <div className="progress-stack">
          <progress
            aria-label="剧集下载进度"
            max={Math.max(progress?.totalCount ?? summary?.episodeCount ?? 1, 1)}
            value={progress?.processedCount ?? 0}
          />
          <p className="progress-caption">{message}</p>
        </div>
      )}

      <button className="submit-button" type="submit" disabled={!canSubmit || busy}>
        {busy ? "下载中..." : "开始下载剧集"}
      </button>

      <p className={`status-line ${tone}`}>{message || "等待选择 TXT 清单目录"}</p>
    </form>
  );
}

function formatSummary(summary: SeriesDownloadListSummary) {
  return `已读取 ${summary.fileCount} 个 TXT：${summary.series.length} 部剧，共 ${summary.episodeCount} 集`;
}

function formatDownloadProgress(progress: SeriesDownloadProgress) {
  const current =
    progress.currentSeries && progress.currentEpisode
      ? `，刚完成 ${progress.currentSeries} 第${progress.currentEpisode}集`
      : "";
  return `下载进度：${progress.processedCount} / ${progress.totalCount}（成功 ${progress.successCount}，失败 ${progress.failureCount}，跳过 ${progress.skippedCount}）${current}`;
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
