import * as Tabs from "@radix-ui/react-tabs";
import { useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

type SplitPdfResult = {
  pageCount: number;
  generatedFileCount: number;
  outputDir: string;
};

type WatermarkPdfResult = {
  outputPdfPath: string;
};

type ExtractImagesResult = {
  outputDir: string;
};

type MessageTone = "idle" | "success" | "error";
type ToolTab = "split" | "extract" | "watermark";
type TabItem = {
  value: ToolTab;
  label: string;
  icon: ReactNode;
};

const TAB_ITEMS: TabItem[] = [
  {
    value: "split",
    label: "按页导出",
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="5" y="4" width="14" height="16" rx="3" />
        <path d="M8 9h8M8 13h8M12 17v-4" />
      </svg>
    ),
  },
  {
    value: "extract",
    label: "提取内嵌图片",
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="4.5" y="5" width="15" height="14" rx="3" />
        <circle cx="9" cy="10" r="1.5" />
        <path d="M7 16l3.5-3.5 2.5 2.5 2.5-3 2.5 4" />
      </svg>
    ),
  },
  {
    value: "watermark",
    label: "文字水印",
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M7 18L12 6l5 12M9 14h6" />
      </svg>
    ),
  },
];

function App() {
  const [activeTab, setActiveTab] = useState<ToolTab>("split");
  const [imagePdfPath, setImagePdfPath] = useState("");
  const [imageOutputDir, setImageOutputDir] = useState("");
  const [imageFormat, setImageFormat] = useState("png");
  const [imageBusy, setImageBusy] = useState(false);
  const [imageMessage, setImageMessage] = useState("");
  const [imageTone, setImageTone] = useState<MessageTone>("idle");

  const [watermarkPdfPath, setWatermarkPdfPath] = useState("");
  const [watermarkOutputDir, setWatermarkOutputDir] = useState("");
  const [watermarkText, setWatermarkText] = useState("");
  const [watermarkBusy, setWatermarkBusy] = useState(false);
  const [watermarkMessage, setWatermarkMessage] = useState("");
  const [watermarkTone, setWatermarkTone] = useState<MessageTone>("idle");

  const [extractPdfPath, setExtractPdfPath] = useState("");
  const [extractOutputDir, setExtractOutputDir] = useState("");
  const [extractBusy, setExtractBusy] = useState(false);
  const [extractMessage, setExtractMessage] = useState("");
  const [extractTone, setExtractTone] = useState<MessageTone>("idle");

  const canSplit = imagePdfPath !== "" && imageOutputDir !== "" && imageFormat !== "";
  const canWatermark =
    watermarkPdfPath !== "" &&
    watermarkOutputDir !== "" &&
    watermarkText.trim() !== "";
  const canExtract = extractPdfPath !== "" && extractOutputDir !== "";

  async function pickPdfFile() {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });

    return normalizeDialogSelection(selected);
  }

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

  async function pickOutputDir() {
    const selected = await open({
      multiple: false,
      directory: true,
    });

    return normalizeDialogSelection(selected);
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
        inputPath: watermarkPdfPath,
        outputDir: watermarkOutputDir,
        watermarkText,
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
    <main className="app-shell">
      <section className="hero">
        <p className="eyebrow">Desktop PDF Workshop</p>
        <h1>PDF 转图片与文字水印</h1>
        <p className="hero-copy">
          在一个页面完成 PDF 按页导出、内嵌图片提取和文字水印生成。文件选择、目录选择和结果回显都在本地执行。
        </p>
      </section>

      <section className="tab-shell">
        <Tabs.Root
          className="tabs-root"
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as ToolTab)}
        >
          <Tabs.List className="tab-strip tab-row" aria-label="PDF 工具切换">
            {TAB_ITEMS.map((item) => (
              <Tabs.Trigger className="tab-button" value={item.value} key={item.value}>
                <span className="tab-icon">{item.icon}</span>
                <span className="tab-label">{item.label}</span>
              </Tabs.Trigger>
            ))}
          </Tabs.List>

          <Tabs.Content className="tab-panel" value="split">
            <form className="tool-card" onSubmit={handleSplitSubmit}>
              <div className="card-head">
                <p className="card-kicker">Tool 01</p>
                <h2>PDF 转图片</h2>
                <p>选择 PDF、选择输出目录，再按页导出为 PNG 或 JPG。</p>
              </div>

              <label className="field">
                <span>PDF 文件</span>
                <div className="picker-row">
                  <input readOnly value={imagePdfPath} placeholder="请选择一个 PDF 文件" />
                  <button type="button" onClick={handlePickImagePdf}>
                    选择 PDF
                  </button>
                </div>
              </label>

              <label className="field">
                <span>输出目录</span>
                <div className="picker-row">
                  <input readOnly value={imageOutputDir} placeholder="请选择输出目录" />
                  <button type="button" onClick={handlePickImageOutputDir}>
                    选择目录
                  </button>
                </div>
              </label>

              <label className="field">
                <span>图片格式</span>
                <select
                  value={imageFormat}
                  onChange={(event) => setImageFormat(event.currentTarget.value)}
                >
                  <option value="png">PNG</option>
                  <option value="jpg">JPG</option>
                </select>
              </label>

              <button className="submit-button" type="submit" disabled={!canSplit || imageBusy}>
                {imageBusy ? "处理中..." : "开始导出图片"}
              </button>

              <p className={`status-line ${imageTone}`}>{imageMessage || "等待执行"}</p>
            </form>
          </Tabs.Content>

          <Tabs.Content className="tab-panel" value="extract">
            <form className="tool-card" onSubmit={handleExtractSubmit}>
              <div className="card-head">
                <p className="card-kicker">Tool 02</p>
                <h2>提取 PDF 内嵌图片</h2>
                <p>提取 PDF 中真正嵌入的图片资源，适合已有扫描图、插图和照片类内容。</p>
              </div>

              <label className="field">
                <span>PDF 文件</span>
                <div className="picker-row">
                  <input readOnly value={extractPdfPath} placeholder="请选择一个 PDF 文件" />
                  <button type="button" onClick={handlePickExtractPdf}>
                    选择 PDF
                  </button>
                </div>
              </label>

              <label className="field">
                <span>输出目录</span>
                <div className="picker-row">
                  <input readOnly value={extractOutputDir} placeholder="请选择输出目录" />
                  <button type="button" onClick={handlePickExtractOutputDir}>
                    选择目录
                  </button>
                </div>
              </label>

              <button
                className="submit-button"
                type="submit"
                disabled={!canExtract || extractBusy}
              >
                {extractBusy ? "处理中..." : "开始提取内嵌图片"}
              </button>

              <p className={`status-line ${extractTone}`}>{extractMessage || "等待执行"}</p>
            </form>
          </Tabs.Content>

          <Tabs.Content className="tab-panel" value="watermark">
            <form className="tool-card" onSubmit={handleWatermarkSubmit}>
            <div className="card-head">
              <p className="card-kicker">Tool 03</p>
              <h2>PDF 文字水印</h2>
              <p>选择 PDF、输入水印文字并输出新的 PDF 文件，不覆盖原文件。</p>
            </div>

            <label className="field">
              <span>PDF 文件</span>
              <div className="picker-row">
                <input readOnly value={watermarkPdfPath} placeholder="请选择一个 PDF 文件" />
                <button type="button" onClick={handlePickWatermarkPdf}>
                  选择 PDF
                </button>
              </div>
            </label>

            <label className="field">
              <span>输出目录</span>
              <div className="picker-row">
                <input
                  readOnly
                  value={watermarkOutputDir}
                  placeholder="请选择输出目录"
                />
                <button type="button" onClick={handlePickWatermarkOutputDir}>
                  选择目录
                </button>
              </div>
            </label>

            <label className="field">
              <span>水印文字</span>
              <textarea
                value={watermarkText}
                placeholder="例如：仅供内部使用"
                onChange={(event) => setWatermarkText(event.currentTarget.value)}
              />
            </label>

            <button
              className="submit-button"
              type="submit"
              disabled={!canWatermark || watermarkBusy}
            >
              {watermarkBusy ? "处理中..." : "开始生成水印 PDF"}
            </button>

            <p className={`status-line ${watermarkTone}`}>
              {watermarkMessage || "等待执行"}
            </p>
            </form>
          </Tabs.Content>
        </Tabs.Root>
      </section>
    </main>
  );
}

function normalizeDialogSelection(selected: string | string[] | null): string | null {
  if (selected == null) {
    return null;
  }

  return Array.isArray(selected) ? (selected[0] ?? null) : selected;
}

export default App;
