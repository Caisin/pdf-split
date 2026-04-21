import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";

import App from "./App";

const {
  openMock,
  invokeMock,
  listenMock,
  setProgressHandler,
  getProgressHandler,
  createObjectUrlMock,
  revokeObjectUrlMock,
  resetObjectUrlCounter,
} = vi.hoisted(
  () => {
    let progressHandler: ((event: { payload: unknown }) => void) | null = null;
    let objectUrlCounter = 0;

    return {
      openMock: vi.fn(),
      invokeMock: vi.fn(),
      listenMock: vi.fn(async (_event: string, handler: (event: { payload: unknown }) => void) => {
        progressHandler = handler;
        return vi.fn();
      }),
      createObjectUrlMock: vi.fn(() => `blob:preview-${++objectUrlCounter}`),
      revokeObjectUrlMock: vi.fn(),
      resetObjectUrlCounter: () => {
        objectUrlCounter = 0;
      },
      setProgressHandler: (handler: ((event: { payload: unknown }) => void) | null) => {
        progressHandler = handler;
      },
      getProgressHandler: () => progressHandler,
    };
  },
);

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openMock,
}));

beforeEach(() => {
  openMock.mockReset();
  invokeMock.mockReset();
  listenMock.mockClear();
  resetObjectUrlCounter();
  createObjectUrlMock.mockClear();
  revokeObjectUrlMock.mockClear();
  setProgressHandler(null);
  URL.createObjectURL = createObjectUrlMock;
  URL.revokeObjectURL = revokeObjectUrlMock;
  vi.useRealTimers();
});

function activateTab(label: string) {
  const tab = screen.getByRole("tab", { name: label });
  fireEvent.mouseDown(tab);
  fireEvent.click(tab);
}

describe("App", () => {
  test("renders top tabs and shows split panel by default", () => {
    render(<App />);

    expect(screen.queryByText("PDF 转图片与文字水印")).not.toBeInTheDocument();
    expect(
      screen.queryByText("在一个页面完成 PDF 按页导出、内嵌图片提取和文字水印生成。文件选择、目录选择和结果回显都在本地执行。"),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("tablist")).toHaveClass("tab-strip");
    expect(screen.getByRole("tab", { name: "按页导出" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "按页导出" })).toHaveAttribute(
      "data-state",
      "active",
    );
    expect(screen.getByRole("tabpanel")).toHaveAttribute("data-state", "active");
    expect(screen.getByRole("tab", { name: "提取内嵌图片" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "文字水印" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "批量图片水印" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "批量视频水印" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "剧集切分" })).toBeInTheDocument();
    expect(screen.getByText("PDF 转图片")).toBeInTheDocument();
    expect(screen.queryByText("PDF 文字水印")).not.toBeInTheDocument();
  });

  test("disables image export submit until required fields are filled", () => {
    render(<App />);

    expect(
      screen.getByRole("button", { name: "开始导出图片" }),
    ).toBeDisabled();
  });

  test("renders split tool with dense compact picker layout", () => {
    render(<App />);

    const heading = screen.getByText("PDF 转图片");
    const form = heading.closest("form");
    expect(form).toHaveClass("tool-card-dense");
    expect(form?.querySelector(".picker-grid")).not.toBeNull();
    expect(form?.querySelector(".field-grid")).not.toBeNull();
  });

  test("disables watermark submit when pdf long edge font ratio is invalid", () => {
    render(<App />);
    activateTab("文字水印");
    fireEvent.change(screen.getByLabelText("PDF 长边字号比例"), {
      target: { value: "0", valueAsNumber: 0 },
    });

    expect(
      screen.getByRole("button", { name: "开始批量生成水印 PDF" }),
    ).toBeDisabled();
  });


  test("submits batch pdf watermark payload through tauri invoke", async () => {
    openMock
      .mockResolvedValueOnce("/tmp/input-pdfs")
      .mockResolvedValueOnce("/tmp/output-dir");
    invokeMock.mockResolvedValue({
      scannedFileCount: 3,
      successCount: 2,
      failureCount: 1,
      outputDir: "/tmp/output-dir",
    });

    render(<App />);
    activateTab("文字水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择输出目录" }));
    await screen.findByDisplayValue("/tmp/input-pdfs");
    await screen.findByDisplayValue("/tmp/output-dir");

    fireEvent.click(screen.getByRole("button", { name: "开始批量生成水印 PDF" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("add_text_watermark_to_pdfs", {
        payload: {
          inputDir: "/tmp/input-pdfs",
          outputDir: "/tmp/output-dir",
          watermarkText: "仅限xxx使用,它用或复印无效",
          watermarkLongEdgeFontRatio: 0.028,
          watermarkOpacity: 0.3,
          watermarkRotationDegrees: -35,
          watermarkStripeGapChars: 2,
          watermarkRowGapLines: 3,
        },
      });
    });
  });

  test("disables batch pdf watermark submit when input and output directories match", async () => {
    openMock.mockResolvedValue("/tmp/pdfs");

    render(<App />);
    activateTab("文字水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择输出目录" }));

    await screen.findByText("输入目录与输出目录不能相同");
    expect(screen.getByRole("button", { name: "开始批量生成水印 PDF" })).toBeDisabled();
  });

  test("shows batch pdf watermark progress and keeps submit disabled while running", async () => {
    openMock
      .mockResolvedValueOnce("/tmp/input-pdfs")
      .mockResolvedValueOnce("/tmp/output-pdfs");

    let resolveInvoke:
      | ((value: {
          scannedFileCount: number;
          successCount: number;
          failureCount: number;
          outputDir: string;
        }) => void)
      | undefined;
    invokeMock.mockImplementation(async (command) => {
      if (command !== "add_text_watermark_to_pdfs") {
        return undefined;
      }

      return await new Promise<{
        scannedFileCount: number;
        successCount: number;
        failureCount: number;
        outputDir: string;
      }>((resolve) => {
        resolveInvoke = resolve;
      });
    });

    render(<App />);
    activateTab("文字水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择输出目录" }));
    await screen.findByDisplayValue("/tmp/input-pdfs");
    await screen.findByDisplayValue("/tmp/output-pdfs");
    fireEvent.click(screen.getByRole("button", { name: "开始批量生成水印 PDF" }));

    expect(await screen.findByRole("button", { name: "处理中..." })).toBeDisabled();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("add_text_watermark_to_pdfs", {
        payload: {
          inputDir: "/tmp/input-pdfs",
          outputDir: "/tmp/output-pdfs",
          watermarkText: "仅限xxx使用,它用或复印无效",
          watermarkLongEdgeFontRatio: 0.028,
          watermarkOpacity: 0.3,
          watermarkRotationDegrees: -35,
          watermarkStripeGapChars: 2,
          watermarkRowGapLines: 3,
        },
      });
    });
    expect(listenMock).toHaveBeenCalledWith(
      "batch-pdf-watermark-progress",
      expect.any(Function),
    );

    await act(async () => {
      getProgressHandler()?.({
        payload: {
          scannedFileCount: 5,
          processedFileCount: 2,
          successCount: 1,
          failureCount: 1,
          currentFile: "nested/demo.pdf",
        },
      });
    });

    expect(await screen.findByRole("progressbar", { name: "PDF 水印处理进度" })).toHaveValue(2);
    expect(
      screen.getAllByText("处理中：2 / 5（成功 1，失败 1）当前文件 nested/demo.pdf"),
    ).toHaveLength(2);

    await act(async () => {
      resolveInvoke?.({
        scannedFileCount: 5,
        successCount: 4,
        failureCount: 1,
        outputDir: "/tmp/output-pdfs",
      });
    });
  });



  test("renders batch image watermark section and keeps submit disabled by default", () => {
    render(<App />);
    activateTab("批量图片水印");

    expect(screen.getByText("批量图片文字水印")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "图片水印参数预览" })).toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "预览图片" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("水印行数")).toHaveValue(10);
    expect(screen.getByLabelText("图片水印透明度")).toHaveValue(0.5);
    expect(screen.getByLabelText("图片水印条间距")).toHaveValue(2);
    expect(screen.getByLabelText("图片水印行间距")).toHaveValue(3);
    expect(screen.getByLabelText("铺满画面")).toBeChecked();
    expect(screen.getByText("直接使用 SlantedWatermarkOptions 参数生成预览与批处理")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "开始批量生成图片水印" }),
    ).toBeDisabled();
  });

  test("renders batch video watermark section and keeps submit disabled by default", () => {
    render(<App />);
    activateTab("批量视频水印");

    const fullscreenSwitch = screen.getByRole("switch", { name: "视频是否平铺" });
    expect(screen.getByText("批量视频文字水印")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "视频水印参数预览" })).toBeInTheDocument();
    expect(screen.getByLabelText("视频水印行数")).toHaveValue(10);
    expect(screen.getByLabelText("视频水印透明度")).toHaveValue(0.3);
    expect(screen.getByLabelText("视频水印倾斜角度")).toHaveValue(-35);
    expect(screen.getByLabelText("视频水印条间距")).toHaveValue(2);
    expect(screen.getByLabelText("视频水印行间距")).toHaveValue(3);
    expect(fullscreenSwitch).toHaveAttribute("aria-checked", "true");
    expect(fullscreenSwitch.closest(".field-grid-compact")).not.toBeNull();
    expect(screen.getByRole("button", { name: "开始批量生成视频水印" })).toBeDisabled();
  });

  test("renders series recut section and keeps submit disabled by default", () => {
    render(<App />);
    activateTab("剧集切分");

    expect(screen.getByRole("heading", { name: "剧集切分" })).toBeInTheDocument();
    expect(screen.getByLabelText("前面保留集数")).toHaveValue(1);
    expect(screen.getByLabelText("目标总集数")).toHaveValue(3);
    expect(screen.getByRole("button", { name: "开始剧集切分" })).toBeDisabled();
  });

  test("shows series recut progress and keeps submit disabled while running", async () => {
    openMock
      .mockResolvedValueOnce("/tmp/input-series")
      .mockResolvedValueOnce("/tmp/output-series");

    let resolveInvoke:
      | ((value: {
          generatedFileCount: number;
          outputDir: string;
          outputFiles: string[];
        }) => void)
      | undefined;
    invokeMock.mockImplementation(async (command) => {
      if (command !== "video_recut_series") {
        return undefined;
      }

      return await new Promise<{
        generatedFileCount: number;
        outputDir: string;
        outputFiles: string[];
      }>((resolve) => {
        resolveInvoke = resolve;
      });
    });

    render(<App />);
    activateTab("剧集切分");

    fireEvent.click(screen.getByRole("button", { name: "选择剧集输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择剧集输出目录" }));
    await screen.findByDisplayValue("/tmp/input-series");
    await screen.findByDisplayValue("/tmp/output-series");
    fireEvent.click(screen.getByRole("button", { name: "开始剧集切分" }));

    expect(await screen.findByRole("button", { name: "处理中..." })).toBeDisabled();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("video_recut_series", {
        payload: {
          inputDir: "/tmp/input-series",
          outputDir: "/tmp/output-series",
          keepCount: 1,
          totalCount: 3,
        },
      });
    });
    expect(listenMock).toHaveBeenCalledWith(
      "series-recut-progress",
      expect.any(Function),
    );

    await act(async () => {
      getProgressHandler()?.({
        payload: {
          totalCount: 3,
          processedCount: 1,
          currentStage: "执行剧集切分",
          currentFile: "01.mp4",
        },
      });
    });

    expect(await screen.findByRole("progressbar", { name: "剧集切分处理进度" })).toHaveValue(1);
    expect(
      screen.getAllByText("处理中：1 / 3（执行剧集切分）当前文件 01.mp4"),
    ).toHaveLength(2);

    await act(async () => {
      resolveInvoke?.({
        generatedFileCount: 3,
        outputDir: "/tmp/output-series",
        outputFiles: [
          "/tmp/output-series/01.mp4",
          "/tmp/output-series/02.mp4",
          "/tmp/output-series/03.mp4",
        ],
      });
    });
  });

  test("loads first video preview frame from the selected input directory", async () => {
    vi.useFakeTimers();
    openMock.mockResolvedValue("/tmp/input-videos");
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "list_input_directory_videos") {
        expect(args).toEqual({ inputDir: "/tmp/input-videos" });
        return {
          files: ["cover.mp4", "nested/demo.mov"],
        };
      }

      if (command === "generate_input_directory_video_preview") {
        expect(args).toEqual({
          payload: {
            inputDir: "/tmp/input-videos",
            relativePath: "cover.mp4",
            watermarkText: "仅限xxx使用,它用或复印无效",
            watermarkLineCount: 10,
            watermarkFullScreen: true,
            watermarkOpacity: 0.3,
            watermarkRotationDegrees: -30,
            watermarkStripeGapChars: 2,
            watermarkRowGapLines: 3,
          },
        });
        return { bytes: [137, 80, 78, 71] };
      }

      return undefined;
    });

    render(<App />);
    activateTab("批量视频水印");
    fireEvent.change(screen.getByLabelText("视频水印倾斜角度"), {
      target: { value: "-30", valueAsNumber: -30 },
    });

    fireEvent.click(screen.getByRole("button", { name: "选择视频输入目录" }));
    await act(async () => {});

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.mp4" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );
  });

  test("regenerates video preview after rotation changes and shows current rotation", async () => {
    vi.useFakeTimers();
    openMock.mockResolvedValue("/tmp/input-videos");
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "list_input_directory_videos") {
        return {
          files: ["cover.mp4"],
        };
      }

      if (command === "generate_input_directory_video_preview") {
        return {
          bytes:
            args?.payload?.watermarkRotationDegrees === -30
              ? [137, 80, 78, 71, 1]
              : [137, 80, 78, 71, 0],
        };
      }

      return undefined;
    });

    render(<App />);
    activateTab("批量视频水印");

    fireEvent.click(screen.getByRole("button", { name: "选择视频输入目录" }));
    await act(async () => {});

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.mp4" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );
    expect(screen.getByText("真实预览：cover.mp4（倾斜角度 -35.0°）")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("视频水印倾斜角度"), {
      target: { value: "-30", valueAsNumber: -30 },
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    const previewCalls = invokeMock.mock.calls.filter(
      ([command]) => command === "generate_input_directory_video_preview",
    );
    expect(previewCalls).toHaveLength(2);
    expect(previewCalls[1]?.[1]).toEqual({
      payload: {
        inputDir: "/tmp/input-videos",
        relativePath: "cover.mp4",
        watermarkText: "仅限xxx使用,它用或复印无效",
        watermarkLineCount: 10,
        watermarkFullScreen: true,
        watermarkOpacity: 0.3,
        watermarkRotationDegrees: -30,
        watermarkStripeGapChars: 2,
        watermarkRowGapLines: 3,
      },
    });
    expect(screen.getByRole("img", { name: "真实预览图：cover.mp4" })).toHaveAttribute(
      "src",
      "blob:preview-2",
    );
    expect(screen.getByText("真实预览：cover.mp4（倾斜角度 -30.0°）")).toBeInTheDocument();
  });

  test("toggles video fullscreen mode through the switch control", () => {
    render(<App />);
    activateTab("批量视频水印");

    const toggle = screen.getByRole("switch", { name: "视频是否平铺" });
    expect(toggle).toHaveAttribute("aria-checked", "true");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-checked", "false");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-checked", "true");
  });

  test("shows batch video watermark progress and keeps submit disabled while running", async () => {
    openMock
      .mockResolvedValueOnce("/tmp/input-videos")
      .mockResolvedValueOnce("/tmp/output-videos");

    let resolveInvoke:
      | ((value: {
          scannedFileCount: number;
          successCount: number;
          generatedOverlayCount: number;
          reusedOverlayCount: number;
          outputDir: string;
        }) => void)
      | undefined;
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_input_directory_videos") {
        return { files: ["cover.mp4"] };
      }
      if (command === "generate_input_directory_video_preview") {
        return { bytes: [137, 80, 78, 71] };
      }
      if (command !== "add_slanted_watermark_to_videos") {
        return undefined;
      }

      return await new Promise<{
        scannedFileCount: number;
        successCount: number;
        generatedOverlayCount: number;
        reusedOverlayCount: number;
        outputDir: string;
      }>((resolve) => {
        resolveInvoke = resolve;
      });
    });

    render(<App />);
    activateTab("批量视频水印");
    fireEvent.change(screen.getByLabelText("视频水印倾斜角度"), {
      target: { value: "-30", valueAsNumber: -30 },
    });

    fireEvent.click(screen.getByRole("button", { name: "选择视频输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择视频输出目录" }));
    await screen.findByDisplayValue("/tmp/input-videos");
    await screen.findByDisplayValue("/tmp/output-videos");
    fireEvent.click(screen.getByRole("button", { name: "开始批量生成视频水印" }));

    expect(await screen.findByRole("button", { name: "处理中..." })).toBeDisabled();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("add_slanted_watermark_to_videos", {
        payload: {
          inputDir: "/tmp/input-videos",
          outputDir: "/tmp/output-videos",
          watermarkText: "仅限xxx使用,它用或复印无效",
          watermarkLineCount: 10,
          watermarkFullScreen: true,
          watermarkOpacity: 0.3,
          watermarkRotationDegrees: -30,
          watermarkStripeGapChars: 2,
          watermarkRowGapLines: 3,
        },
      });
    });
    expect(listenMock).toHaveBeenCalledWith(
      "batch-video-watermark-progress",
      expect.any(Function),
    );

    await act(async () => {
      getProgressHandler()?.({
        payload: {
          scannedFileCount: 6,
          processedFileCount: 2,
          successCount: 2,
          generatedOverlayCount: 1,
          reusedOverlayCount: 1,
          currentFile: "nested/demo.mp4",
        },
      });
    });

    expect(await screen.findByRole("progressbar", { name: "视频水印处理进度" })).toHaveValue(2);
    expect(
      screen.getAllByText("处理中：2 / 6（成功 2，新增水印图 1，复用水印图 1）当前文件 nested/demo.mp4"),
    ).toHaveLength(2);

    await act(async () => {
      resolveInvoke?.({
        scannedFileCount: 6,
        successCount: 6,
        generatedOverlayCount: 2,
        reusedOverlayCount: 4,
        outputDir: "/tmp/output-videos",
      });
    });
  });

  test("loads previewable images from the selected input directory and previews the chosen image", async () => {
    vi.useFakeTimers();
    openMock.mockResolvedValue("/tmp/input-images");
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "list_input_directory_images") {
        expect(args).toEqual({ inputDir: "/tmp/input-images" });
        return {
          files: ["cover.png", "nested/demo.jpg"],
        };
      }

      if (command === "generate_input_directory_image_preview") {
        if (args?.payload?.relativePath === "cover.png") {
          return { bytes: [137, 80, 78, 71] };
        }

        if (args?.payload?.relativePath === "nested/demo.jpg") {
          return { bytes: [255, 216, 255, 224] };
        }
      }

      return undefined;
    });

    render(<App />);
    activateTab("批量图片水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));

    await act(async () => {});

    const previewPicker = screen.getByRole("combobox", { name: "预览图片" });
    expect(previewPicker).toHaveValue("cover.png");
    expect(invokeMock).not.toHaveBeenCalledWith(
      "generate_input_directory_image_preview",
      expect.anything(),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.png" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );

    fireEvent.change(previewPicker, { target: { value: "nested/demo.jpg" } });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(
      screen.getByRole("img", { name: "真实预览图：nested/demo.jpg" }),
    ).toHaveAttribute("src", "blob:preview-2");
  });

  test("keeps the last preview visible while updating and ignores stale preview results", async () => {
    vi.useFakeTimers();
    openMock.mockResolvedValue("/tmp/input-images");
    let resolveSecondPreview: ((value: { bytes: number[] }) => void) | undefined;
    let resolveThirdPreview: ((value: { bytes: number[] }) => void) | undefined;
    let previewCallCount = 0;
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_input_directory_images") {
        return { files: ["cover.png"] };
      }

      if (command === "generate_input_directory_image_preview") {
        previewCallCount += 1;
        if (previewCallCount === 1) {
          return { bytes: [137, 80, 78, 71] };
        }

        if (previewCallCount === 2) {
          return await new Promise<{ bytes: number[] }>((resolve) => {
            resolveSecondPreview = resolve;
          });
        }

        if (previewCallCount === 3) {
          return await new Promise<{ bytes: number[] }>((resolve) => {
            resolveThirdPreview = resolve;
          });
        }
      }

      return undefined;
    });

    render(<App />);
    activateTab("批量图片水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    await act(async () => {});
    expect(screen.getByRole("combobox", { name: "预览图片" })).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.png" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );

    fireEvent.change(screen.getByRole("textbox", { name: "水印文字" }), {
      target: { value: "测试预览文字" },
    });
    fireEvent.change(screen.getByLabelText("水印行数"), {
      target: { value: "4", valueAsNumber: 4 },
    });
    fireEvent.change(screen.getByLabelText("图片水印透明度"), {
      target: { value: "0.35", valueAsNumber: 0.35 },
    });
    fireEvent.click(screen.getByLabelText("铺满画面"));
    fireEvent.change(screen.getByLabelText("图片水印条间距"), {
      target: { value: "1.5", valueAsNumber: 1.5 },
    });
    fireEvent.change(screen.getByLabelText("图片水印行间距"), {
      target: { value: "2.5", valueAsNumber: 2.5 },
    });

    expect(
      invokeMock.mock.calls.filter(([command]) => command === "generate_input_directory_image_preview"),
    ).toHaveLength(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(399);
    });

    expect(
      invokeMock.mock.calls.filter(([command]) => command === "generate_input_directory_image_preview"),
    ).toHaveLength(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.png" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );
    expect(screen.getByText("更新中")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenLastCalledWith(
      "generate_input_directory_image_preview",
      {
        payload: {
          inputDir: "/tmp/input-images",
          relativePath: "cover.png",
          watermarkText: "测试预览文字",
          watermarkLineCount: 4,
          watermarkFullScreen: false,
          watermarkOpacity: 0.35,
          watermarkStripeGapChars: 1.5,
          watermarkRowGapLines: 2.5,
        },
      },
    );

    fireEvent.change(screen.getByLabelText("水印行数"), {
      target: { value: "5", valueAsNumber: 5 },
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    await act(async () => {
      resolveSecondPreview?.({ bytes: [255, 216, 255, 224] });
    });
    expect(screen.getByRole("img", { name: "真实预览图：cover.png" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );

    await act(async () => {
      resolveThirdPreview?.({ bytes: [71, 73, 70, 56] });
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.png" })).toHaveAttribute(
      "src",
      "blob:preview-2",
    );
  });

  test("disables batch image watermark submit when input and output directories match", async () => {
    openMock.mockResolvedValue("/tmp/images");

    render(<App />);
    activateTab("批量图片水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择输出目录" }));

    await screen.findByText("输入目录与输出目录不能相同");
    expect(
      screen.getByRole("button", { name: "开始批量生成图片水印" }),
    ).toBeDisabled();
  });

  test("shows batch image watermark progress and keeps submit disabled while running", async () => {
    openMock
      .mockResolvedValueOnce("/tmp/input-images")
      .mockResolvedValueOnce("/tmp/output-images");

    let resolveInvoke:
      | ((value: {
          scannedFileCount: number;
          successCount: number;
          failureCount: number;
          outputDir: string;
        }) => void)
      | undefined;
    invokeMock.mockImplementation(
      async (command) => {
        if (command === "list_input_directory_images") {
          return { files: ["cover.png"] };
        }

        if (command === "generate_input_directory_image_preview") {
          return { bytes: [137, 80, 78, 71] };
        }

        return await new Promise<{
          scannedFileCount: number;
          successCount: number;
          failureCount: number;
          outputDir: string;
        }>((resolve) => {
          resolveInvoke = resolve;
        });
      },
    );

    render(<App />);
    activateTab("批量图片水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择输出目录" }));
    await screen.findByDisplayValue("/tmp/input-images");
    await screen.findByDisplayValue("/tmp/output-images");
    fireEvent.click(screen.getByRole("button", { name: "开始批量生成图片水印" }));

    expect(await screen.findByRole("button", { name: "处理中..." })).toBeDisabled();
    await act(async () => {});
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("add_text_watermark_to_images", {
        payload: {
          inputDir: "/tmp/input-images",
          outputDir: "/tmp/output-images",
          watermarkText: "仅限xxx使用,它用或复印无效",
          watermarkLineCount: 10,
          watermarkFullScreen: true,
          watermarkOpacity: 0.5,
          watermarkStripeGapChars: 2,
          watermarkRowGapLines: 3,
        },
      });
    });
    expect(listenMock).toHaveBeenCalledWith(
      "batch-image-watermark-progress",
      expect.any(Function),
    );

    await act(async () => {
      getProgressHandler()?.({
        payload: {
          scannedFileCount: 10,
          processedFileCount: 3,
          successCount: 2,
          failureCount: 1,
          currentFile: "nested/demo.png",
        },
      });
    });

    expect(await screen.findByRole("progressbar", { name: "图片水印处理进度" })).toHaveValue(3);
    expect(
      screen.getAllByText("处理中：3 / 10（成功 2，失败 1）当前文件 nested/demo.png"),
    ).toHaveLength(2);

    await act(async () => {
      resolveInvoke?.({
        scannedFileCount: 10,
        successCount: 9,
        failureCount: 1,
        outputDir: "/tmp/output-images",
      });
    });
  });


  test("renders embedded image extraction section and keeps submit disabled by default", () => {
    render(<App />);
    activateTab("提取内嵌图片");

    expect(screen.getByText("提取 PDF 内嵌图片")).toBeInTheDocument();
    const form = screen.getByText("提取 PDF 内嵌图片").closest("form");
    expect(form).toHaveClass("tool-card-dense");
    expect(form?.querySelector(".picker-grid")).not.toBeNull();
    expect(
      screen.getByRole("button", { name: "开始提取内嵌图片" }),
    ).toBeDisabled();
  });

  test("uses frontend dialog API when choosing a PDF file", () => {
    openMock.mockResolvedValue("/tmp/demo.pdf");

    render(<App />);
    fireEvent.click(screen.getAllByRole("button", { name: "选择 PDF" })[0]);

    expect(openMock).toHaveBeenCalledWith(
      expect.objectContaining({
        multiple: false,
        directory: false,
      }),
    );
  });

  test("switches tools through top tabs", () => {
    render(<App />);

    activateTab("提取内嵌图片");
    expect(screen.getByText("提取 PDF 内嵌图片")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "提取内嵌图片" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    activateTab("文字水印");
    expect(screen.getByText("批量 PDF 文字水印")).toBeInTheDocument();
    expect(screen.queryByText("提取 PDF 内嵌图片")).not.toBeInTheDocument();
  });

  test("prefills watermark text and exposes latest upstream pdf watermark controls", () => {
    render(<App />);
    activateTab("文字水印");

    const form = screen.getByText("批量 PDF 文字水印").closest("form");
    expect(form).toHaveClass("tool-card-dense");
    expect(form?.querySelector(".picker-grid")).not.toBeNull();
    expect(form?.querySelector(".field-grid-compact")).not.toBeNull();
    expect(screen.getByRole("img", { name: "PDF 水印参数预览" })).toBeInTheDocument();
    expect(
      screen.getByDisplayValue("仅限xxx使用,它用或复印无效"),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("PDF 长边字号比例")).toHaveValue(0.028);
    expect(screen.getByLabelText("PDF 水印透明度")).toHaveValue(0.3);
    expect(screen.getByLabelText("PDF 水印倾斜角度")).toHaveValue(-35);
    expect(screen.getByLabelText("PDF 水印条间距")).toHaveValue(2);
    expect(screen.getByLabelText("PDF 水印行间距")).toHaveValue(3);
    expect(screen.getByRole("button", { name: "开始批量生成水印 PDF" })).toBeDisabled();
  });

  test("loads first pdf page preview from the selected input directory", async () => {
    vi.useFakeTimers();
    openMock.mockResolvedValue("/tmp/input-pdfs");
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "list_input_directory_pdfs") {
        expect(args).toEqual({ inputDir: "/tmp/input-pdfs" });
        return {
          files: ["cover.pdf", "nested/demo.pdf"],
        };
      }

      if (command === "generate_input_directory_pdf_preview") {
        expect(args).toEqual({
          payload: {
            inputDir: "/tmp/input-pdfs",
            relativePath: "cover.pdf",
            watermarkText: "仅限xxx使用,它用或复印无效",
            watermarkLongEdgeFontRatio: 0.028,
            watermarkOpacity: 0.3,
            watermarkRotationDegrees: -35,
            watermarkStripeGapChars: 2,
            watermarkRowGapLines: 3,
          },
        });
        return { bytes: [137, 80, 78, 71] };
      }

      return undefined;
    });

    render(<App />);
    activateTab("文字水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    await act(async () => {});

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(screen.getByRole("img", { name: "真实预览图：cover.pdf" })).toHaveAttribute(
      "src",
      "blob:preview-1",
    );
    expect(screen.getByText("真实预览：cover.pdf（倾斜角度 -35.0°）")).toBeInTheDocument();
  });
});
