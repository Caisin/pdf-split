export type SplitPdfResult = {
  pageCount: number;
  generatedFileCount: number;
  outputDir: string;
};

export type WatermarkPdfResult = {
  outputPdfPath: string;
};

export type ExtractImagesResult = {
  outputDir: string;
};

export type BatchImageWatermarkResult = {
  scannedFileCount: number;
  successCount: number;
  failureCount: number;
  outputDir: string;
};

export type BatchImageWatermarkProgress = {
  scannedFileCount: number;
  processedFileCount: number;
  successCount: number;
  failureCount: number;
  currentFile: string | null;
};

export type MessageTone = "idle" | "success" | "error";
