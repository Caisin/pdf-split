import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { PickerField } from "../common/PickerField";
import { pathsLookSame, pickOutputDir } from "../common/dialog";
import type { MessageTone, SeriesRecutProgress, SeriesRecutResult } from "../tool-types";

export function SeriesRecutTool() {
  const [inputDir, setInputDir] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [keepCount, setKeepCount] = useState(1);
  const [totalCount, setTotalCount] = useState(3);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<SeriesRecutProgress | null>(null);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState<MessageTone>("idle");

  const directoryConflict = inputDir !== "" && outputDir !== "" && pathsLookSame(inputDir, outputDir);
  const canSubmit =
    inputDir !== "" &&
    outputDir !== "" &&
    !directoryConflict &&
    Number.isInteger(keepCount) &&
    keepCount >= 0 &&
    Number.isInteger(totalCount) &&
    totalCount >= keepCount;

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
      unlistenProgress = await listen<SeriesRecutProgress>("series-recut-progress", ({ payload }) => {
        setProgress(payload);
        setMessage(formatSeriesRecutProgress(payload));
      });
      await yieldToBrowser();

      const result = await invoke<SeriesRecutResult>("video_recut_series", {
        payload: {
          inputDir,
          outputDir,
          keepCount,
          totalCount,
        },
      });

      setProgress(null);
      setTone("success");
      setMessage(`完成：共生成 ${result.generatedFileCount} 集，输出目录 ${result.outputDir}`);
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
        <p className="card-kicker">Tool 06</p>
        <h2>剧集切分</h2>
        <p>按纯数字命名的剧集目录，保留前几集后，将剩余内容重组切分为新的总集数。</p>
      </div>

      <div className="picker-grid">
        <PickerField
          label="输入目录"
          placeholder="请选择按数字命名的剧集目录"
          value={inputDir}
          buttonLabel="选择剧集输入目录"
          kind="folder"
          onPick={handlePickInputDir}
        />

        <PickerField
          label="输出目录"
          placeholder="请选择输出目录"
          value={outputDir}
          buttonLabel="选择剧集输出目录"
          kind="folder"
          onPick={handlePickOutputDir}
        />
      </div>

      <div className="field-grid">
        <label className="field">
          <span>前面保留集数</span>
          <div className="input-shell">
            <input
              aria-label="前面保留集数"
              type="number"
              min={0}
              step={1}
              value={keepCount}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setKeepCount(Number.isFinite(nextValue) ? Math.trunc(nextValue) : -1);
              }}
            />
          </div>
        </label>

        <label className="field">
          <span>目标总集数</span>
          <div className="input-shell">
            <input
              aria-label="目标总集数"
              type="number"
              min={0}
              step={1}
              value={totalCount}
              onChange={(event) => {
                const nextValue = event.currentTarget.valueAsNumber;
                setTotalCount(Number.isFinite(nextValue) ? Math.trunc(nextValue) : -1);
              }}
            />
          </div>
        </label>
      </div>

      <p className={`status-line ${directoryConflict ? "error" : "idle"}`}>
        {directoryConflict
          ? "输入目录与输出目录不能相同"
          : "默认使用上游 CleanPreview 模式，避免输出文件预览首帧发黑"}
      </p>

      {busy && (
        <div className="progress-stack">
          <progress
            aria-label="剧集切分处理进度"
            max={Math.max(progress?.totalCount ?? 1, 1)}
            value={progress?.processedCount ?? 0}
          />
          <p className="progress-caption">{message || "处理中..."}</p>
        </div>
      )}

      <button className="submit-button" type="submit" disabled={!canSubmit || busy}>
        {busy ? "处理中..." : "开始剧集切分"}
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

function formatSeriesRecutProgress(progress: SeriesRecutProgress) {
  const currentFile = progress.currentFile ? `当前文件 ${progress.currentFile}` : "正在准备文件列表";
  return `处理中：${progress.processedCount} / ${progress.totalCount}（${progress.currentStage}）${currentFile}`;
}
