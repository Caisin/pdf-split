import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";

import App from "./App";

const { openMock, invokeMock } = vi.hoisted(() => ({
  openMock: vi.fn(),
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openMock,
}));

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
