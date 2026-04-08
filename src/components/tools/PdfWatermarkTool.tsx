import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { PickerField } from "../common/PickerField";
import { pickOutputDir, pickPdfFile } from "../common/dialog";
import type { MessageTone, WatermarkPdfResult } from "../tool-types";

const DEFAULT_WATERMARK_TEXT = "仅限xxx使用,它用或复印无效";

export function PdfWatermarkTool() {
  const [watermarkPdfPath, setWatermarkPdfPath] = useState("");
  const [watermarkOutputDir, setWatermarkOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState(DEFAULT_WATERMARK_TEXT);
  const [watermarkFontSize, setWatermarkFontSize] = useState(28);
  const [watermarkBusy, setWatermarkBusy] = useState(false);
  const [watermarkMessage, setWatermarkMessage] = useState("");
  const [watermarkTone, setWatermarkTone] = useState<MessageTone>("idle");

  const canWatermark =
    watermarkPdfPath !== "" &&
    watermarkOutputDir !== "" &&
    watermarkText.trim() !== "" &&
    Number.isFinite(watermarkFontSize) &&
    watermarkFontSize > 0;

  async function handlePickWatermarkPdf() {
    const selected = await pickPdfFile();
    if (selected) {
      setWatermarkPdfPath(selected);
      setWatermarkMessage("");
      setWatermarkTone("idle");
    }
  }

  async function handlePickWatermarkOutputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setWatermarkOutputDir(selected);
      setWatermarkMessage("");
      setWatermarkTone("idle");
    }
  }

  async function handleWatermarkSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canWatermark || watermarkBusy) {
      return;
    }

    setWatermarkBusy(true);
    setWatermarkMessage("处理中...");
    setWatermarkTone("idle");

    try {
      const result = await invoke<WatermarkPdfResult>("add_text_watermark", {
        payload: {
          inputPath: watermarkPdfPath,
          outputDir: watermarkOutputDir,
          watermarkText,
          watermarkFontSize,
        },
      });
      setWatermarkTone("success");
      setWatermarkMessage(`完成：输出文件 ${result.outputPdfPath}`);
    } catch (error) {
      setWatermarkTone("error");
      setWatermarkMessage(String(error));
    } finally {
      setWatermarkBusy(false);
    }
  }

  return (
    <form className="tool-card" onSubmit={handleWatermarkSubmit}>
      <div className="card-head">
        <p className="card-kicker">Tool 03</p>
        <h2>PDF 文字水印</h2>
        <p>选择 PDF、输入水印文字并输出新的 PDF 文件，不覆盖原文件。</p>
      </div>

      <PickerField
        label="PDF 文件"
        placeholder="请选择一个 PDF 文件"
        value={watermarkPdfPath}
        buttonLabel="选择 PDF"
        kind="file"
        onPick={handlePickWatermarkPdf}
      />

      <PickerField
        label="输出目录"
        placeholder="请选择输出目录"
        value={watermarkOutputDir}
        buttonLabel="选择目录"
        kind="folder"
        onPick={handlePickWatermarkOutputDir}
      />

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

      <button className="submit-button" type="submit" disabled={!canWatermark || watermarkBusy}>
        {watermarkBusy ? "处理中..." : "开始生成水印 PDF"}
      </button>

      <p className={`status-line ${watermarkTone}`}>{watermarkMessage || "等待执行"}</p>
    </form>
  );
}
