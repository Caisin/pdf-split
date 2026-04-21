import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { PickerField } from "../common/PickerField";
import { pickOutputDir, pickPdfFile } from "../common/dialog";
import type { MessageTone, SplitPdfResult } from "../tool-types";

export function SplitPdfTool() {
  const [imagePdfPath, setImagePdfPath] = useState("");
  const [imageOutputDir, setImageOutputDir] = useState("");
  const [imageFormat, setImageFormat] = useState("png");
  const [imageBusy, setImageBusy] = useState(false);
  const [imageMessage, setImageMessage] = useState("");
  const [imageTone, setImageTone] = useState<MessageTone>("idle");

  const canSplit = imagePdfPath !== "" && imageOutputDir !== "" && imageFormat !== "";

  async function handlePickImagePdf() {
    const selected = await pickPdfFile();
    if (selected) {
      setImagePdfPath(selected);
      setImageMessage("");
      setImageTone("idle");
    }
  }

  async function handlePickImageOutputDir() {
    const selected = await pickOutputDir();
    if (selected) {
      setImageOutputDir(selected);
      setImageMessage("");
      setImageTone("idle");
    }
  }

  async function handleSplitSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSplit || imageBusy) {
      return;
    }

    setImageBusy(true);
    setImageMessage("处理中...");
    setImageTone("idle");

    try {
      const result = await invoke<SplitPdfResult>("split_pdf_to_images", {
        inputPath: imagePdfPath,
        outputDir: imageOutputDir,
        imageFormat,
      });
      setImageTone("success");
      setImageMessage(
        `完成：共 ${result.pageCount} 页，生成 ${result.generatedFileCount} 张图片，输出目录 ${result.outputDir}`,
      );
    } catch (error) {
      setImageTone("error");
      setImageMessage(String(error));
    } finally {
      setImageBusy(false);
    }
  }

  return (
    <form className="tool-card tool-card-dense" onSubmit={handleSplitSubmit}>
      <div className="card-head">
        <p className="card-kicker">Tool 01</p>
        <h2>PDF 转图片</h2>
        <p>选择 PDF、选择输出目录，再按页导出为 PNG 或 JPG。</p>
      </div>

      <div className="picker-grid">
        <PickerField
          label="PDF 文件"
          placeholder="请选择一个 PDF 文件"
          value={imagePdfPath}
          buttonLabel="选择 PDF"
          kind="file"
          onPick={handlePickImagePdf}
        />

        <PickerField
          label="输出目录"
          placeholder="请选择输出目录"
          value={imageOutputDir}
          buttonLabel="选择目录"
          kind="folder"
          onPick={handlePickImageOutputDir}
        />
      </div>

      <div className="field-grid">
        <label className="field">
          <span>图片格式</span>
          <div className="input-shell">
            <select value={imageFormat} onChange={(event) => setImageFormat(event.currentTarget.value)}>
              <option value="png">PNG</option>
              <option value="jpg">JPG</option>
            </select>
          </div>
        </label>
      </div>

      <button className="submit-button" type="submit" disabled={!canSplit || imageBusy}>
        {imageBusy ? "处理中..." : "开始导出图片"}
      </button>

      <p className={`status-line ${imageTone}`}>{imageMessage || "等待执行"}</p>
    </form>
  );
}
