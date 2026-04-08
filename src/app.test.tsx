import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";

import App from "./App";

const { openMock, invokeMock, listenMock, setProgressHandler, getProgressHandler } = vi.hoisted(
  () => {
    let progressHandler: ((event: { payload: unknown }) => void) | null = null;

    return {
      openMock: vi.fn(),
      invokeMock: vi.fn(),
      listenMock: vi.fn(async (_event: string, handler: (event: { payload: unknown }) => void) => {
        progressHandler = handler;
        return vi.fn();
      }),
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
  setProgressHandler(null);
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



  test("renders batch image watermark section and keeps submit disabled by default", () => {
    render(<App />);
    activateTab("批量图片水印");

    expect(screen.getByText("批量图片文字水印")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "开始批量生成图片水印" }),
    ).toBeDisabled();
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
      () =>
        new Promise<{
          scannedFileCount: number;
          successCount: number;
          failureCount: number;
          outputDir: string;
        }>((resolve) => {
          resolveInvoke = resolve;
        }),
    );

    render(<App />);
    activateTab("批量图片水印");

    fireEvent.click(screen.getByRole("button", { name: "选择输入目录" }));
    fireEvent.click(screen.getByRole("button", { name: "选择输出目录" }));
    await screen.findByDisplayValue("/tmp/input-images");
    await screen.findByDisplayValue("/tmp/output-images");
    fireEvent.click(screen.getByRole("button", { name: "开始批量生成图片水印" }));

    expect(await screen.findByRole("button", { name: "处理中..." })).toBeDisabled();
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
