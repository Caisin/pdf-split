import { act, fireEvent, render, screen } from "@testing-library/react";
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
    expect(screen.getByText("PDF 转图片")).toBeInTheDocument();
    expect(screen.queryByText("PDF 文字水印")).not.toBeInTheDocument();
  });

  test("disables image export submit until required fields are filled", () => {
    render(<App />);

    expect(
      screen.getByRole("button", { name: "开始导出图片" }),
    ).toBeDisabled();
  });

  test("disables watermark submit when watermark font size is invalid", () => {
    render(<App />);
    activateTab("文字水印");
    fireEvent.change(screen.getByLabelText("水印字号"), {
      target: { value: "0", valueAsNumber: 0 },
    });

    expect(
      screen.getByRole("button", { name: "开始生成水印 PDF" }),
    ).toBeDisabled();
  });


  test("submits pdf watermark payload through tauri invoke", async () => {
    openMock
      .mockResolvedValueOnce("/tmp/demo.pdf")
      .mockResolvedValueOnce("/tmp/output-dir");
    invokeMock.mockResolvedValue({ outputPdfPath: "/tmp/output-dir/demo-watermarked.pdf" });

    render(<App />);
    activateTab("文字水印");

    fireEvent.click(screen.getByRole("button", { name: "选择 PDF" }));
    fireEvent.click(screen.getByRole("button", { name: "选择目录" }));
    await screen.findByDisplayValue("/tmp/demo.pdf");
    await screen.findByDisplayValue("/tmp/output-dir");

    fireEvent.click(screen.getByRole("button", { name: "开始生成水印 PDF" }));

    expect(invokeMock).toHaveBeenCalledWith("add_text_watermark", {
      payload: {
        inputPath: "/tmp/demo.pdf",
        outputDir: "/tmp/output-dir",
        watermarkText: "仅限xxx使用,它用或复印无效",
        watermarkFontSize: 28,
      },
    });
  });



  test("renders batch image watermark section and keeps submit disabled by default", () => {
    render(<App />);
    activateTab("批量图片水印");

    expect(screen.getByText("批量图片文字水印")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "图片水印参数预览" })).toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "预览图片" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("长边字号比例")).toHaveValue(2.8);
    expect(screen.getByLabelText("图片水印横向间距比例")).toHaveValue(18);
    expect(screen.getByLabelText("图片水印纵向间距比例")).toHaveValue(12);
    expect(
      screen.getByText("字号将按图片长边自动计算，并限制在短边的 1% - 50% 之间"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "开始批量生成图片水印" }),
    ).toBeDisabled();
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
    fireEvent.change(screen.getByLabelText("长边字号比例"), {
      target: { value: "3.5", valueAsNumber: 3.5 },
    });
    fireEvent.change(screen.getByLabelText("图片水印透明度"), {
      target: { value: "42", valueAsNumber: 42 },
    });
    fireEvent.change(screen.getByLabelText("图片水印旋转角度"), {
      target: { value: "-20", valueAsNumber: -20 },
    });
    fireEvent.change(screen.getByLabelText("图片水印横向间距比例"), {
      target: { value: "22", valueAsNumber: 22 },
    });
    fireEvent.change(screen.getByLabelText("图片水印纵向间距比例"), {
      target: { value: "14", valueAsNumber: 14 },
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
          watermarkLongEdgeFontRatio: 3.5,
          watermarkOpacity: 42,
          watermarkRotation: -20,
          watermarkHorizontalSpacingRatio: 22,
          watermarkVerticalSpacingRatio: 14,
        },
      },
    );

    fireEvent.change(screen.getByLabelText("长边字号比例"), {
      target: { value: "4.2", valueAsNumber: 4.2 },
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
    expect(invokeMock).toHaveBeenCalledWith("add_text_watermark_to_images", {
      payload: {
        inputDir: "/tmp/input-images",
        outputDir: "/tmp/output-images",
        watermarkText: "仅限xxx使用,它用或复印无效",
        watermarkLongEdgeFontRatio: 2.8,
        watermarkOpacity: 18,
        watermarkRotation: -35,
        watermarkHorizontalSpacingRatio: 18,
        watermarkVerticalSpacingRatio: 12,
      },
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
    expect(screen.getByText("PDF 文字水印")).toBeInTheDocument();
    expect(screen.queryByText("提取 PDF 内嵌图片")).not.toBeInTheDocument();
  });

  test("prefills watermark text and exposes font size control", () => {
    render(<App />);
    activateTab("文字水印");

    expect(
      screen.getByDisplayValue("仅限xxx使用,它用或复印无效"),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("水印字号")).toHaveValue(28);
  });
});
