import type { ReactNode } from "react";

export type ToolTab =
  | "split"
  | "extract"
  | "watermark"
  | "imageWatermark"
  | "videoWatermark"
  | "seriesRecut";

type TabItem = {
  value: ToolTab;
  label: string;
  icon: ReactNode;
};

export const TAB_ITEMS: TabItem[] = [
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
  {
    value: "imageWatermark",
    label: "批量图片水印",
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="4.5" y="5" width="15" height="14" rx="3" />
        <path d="M7 16l10-8" />
        <path d="M8.5 18.5l7-12" />
      </svg>
    ),
  },
  {
    value: "videoWatermark",
    label: "批量视频水印",
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="4.5" y="5" width="15" height="14" rx="3" />
        <path d="M10 9.5l5 2.5-5 2.5z" />
        <path d="M7 18l10-12" />
      </svg>
    ),
  },
  {
    value: "seriesRecut",
    label: "剧集切分",
    icon: (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="4.5" y="5" width="15" height="14" rx="3" />
        <path d="M9 8.5h6M9 12h6M9 15.5h4" />
        <path d="M14 18l4-4" />
      </svg>
    ),
  },
];
