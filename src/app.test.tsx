import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";

import App from "./App";

const { openMock } = vi.hoisted(() => ({
  openMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openMock,
}));

describe("App", () => {
  test("renders top tabs and shows split panel by default", () => {
    render(<App />);

    expect(screen.getByRole("tab", { name: "按页导出" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
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

  test("disables watermark submit when watermark text is empty", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("tab", { name: "文字水印" }));

    expect(
      screen.getByRole("button", { name: "开始生成水印 PDF" }),
    ).toBeDisabled();
  });

  test("renders embedded image extraction section and keeps submit disabled by default", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("tab", { name: "提取内嵌图片" }));

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

    fireEvent.click(screen.getByRole("tab", { name: "提取内嵌图片" }));
    expect(screen.getByText("提取 PDF 内嵌图片")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "提取内嵌图片" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    fireEvent.click(screen.getByRole("tab", { name: "文字水印" }));
    expect(screen.getByText("PDF 文字水印")).toBeInTheDocument();
    expect(screen.queryByText("提取 PDF 内嵌图片")).not.toBeInTheDocument();
  });
});
