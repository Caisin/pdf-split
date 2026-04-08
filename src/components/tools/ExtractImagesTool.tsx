import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { PickerField } from "../common/PickerField";
import { pickOutputDir, pickPdfFile } from "../common/dialog";
import type { ExtractImagesResult, MessageTone } from "../tool-types";

export function ExtractImagesTool() {
  const [extractPdfPath, setExtractPdfPath] = useState("");
  const [extractOutputDir, setExtractOutputDir] = useState("");
  const [extractBusy, setExtractBusy] = useState(false);
  const [extractMessage, setExtractMessage] = useState("");
  const [extractTone, setExtractTone] = useState<MessageTone>("idle");

  const canExtract = extractPdfPath !== "" && extractOutputDir !== "";

  async function handlePickExtractPdf() {
    const selected = await pickPdfFile();
    if (selected) {
      setExtractPdfPath(selected);
      setExtractMessage("");
      setExtractTone("idle");
    }
  }

  async function handlePickExtractOutputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setExtractOutputDir(selected);
      setExtractMessage("");
      setExtractTone("idle");
    }
  }

  async function handleExtractSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canExtract || extractBusy) {
      return;
    }

    setExtractBusy(true);
    setExtractMessage("处理中...");
    setExtractTone("idle");

    try {
      const result = await invoke<ExtractImagesResult>("extract_embedded_images", {
        inputPath: extractPdfPath,
        outputDir: extractOutputDir,
      });
      setExtractTone("success");
      setExtractMessage(`完成：已提取 PDF 内嵌图片，输出目录 ${result.outputDir}`);
    } catch (error) {
      setExtractTone("error");
      setExtractMessage(String(error));
    } finally {
      setExtractBusy(false);
    }
  }

  return (
    <form className="tool-card" onSubmit={handleExtractSubmit}>
      <div className="card-head">
        <p className="card-kicker">Tool 02</p>
        <h2>提取 PDF 内嵌图片</h2>
        <p>提取 PDF 中真正嵌入的图片资源，适合已有扫描图、插图和照片类内容。</p>
      </div>

      <PickerField
        label="PDF 文件"
        placeholder="请选择一个 PDF 文件"
        value={extractPdfPath}
        buttonLabel="选择 PDF"
        kind="file"
        onPick={handlePickExtractPdf}
      />

      <PickerField
        label="输出目录"
        placeholder="请选择输出目录"
        value={extractOutputDir}
        buttonLabel="选择目录"
        kind="folder"
        onPick={handlePickExtractOutputDir}
      />

      <button className="submit-button" type="submit" disabled={!canExtract || extractBusy}>
        {extractBusy ? "处理中..." : "开始提取内嵌图片"}
      </button>

      <p className={`status-line ${extractTone}`}>{extractMessage || "等待执行"}</p>
    </form>
  );
}
