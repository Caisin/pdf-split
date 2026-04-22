export type SplitPdfResult = {
  pageCount: number;
  generatedFileCount: number;
  outputDir: string;
};

export type WatermarkPdfResult = {
  outputPdfPath: string;
};

export type BatchPdfWatermarkResult = {
  scannedFileCount: number;
  successCount: number;
  failureCount: number;
  outputDir: string;
};

export type InputDirectoryPdfListResult = {
  files: string[];
};

export type BatchPdfWatermarkProgress = {
  scannedFileCount: number;
  processedFileCount: number;
  successCount: number;
  failureCount: number;
  currentFile: string | null;
};

export type BatchVideoWatermarkResult = {
  scannedFileCount: number;
  successCount: number;
  generatedOverlayCount: number;
  reusedOverlayCount: number;
  outputDir: string;
};

export type BatchVideoWatermarkProgress = {
  scannedFileCount: number;
  processedFileCount: number;
  successCount: number;
  generatedOverlayCount: number;
  reusedOverlayCount: number;
  currentFile: string | null;
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

export type InputDirectoryImageListResult = {
  files: string[];
};

export type InputDirectoryVideoListResult = {
  files: string[];
};

export type PreviewImageBytesResult = {
  bytes: number[];
};

export type SeriesRecutResult = {
  generatedFileCount: number;
  outputDir: string;
  outputFiles: string[];
};

export type SeriesRecutProgress = {
  totalCount: number;
  processedCount: number;
  currentStage: string;
  currentFile: string | null;
};

export type MessageTone = "idle" | "success" | "error";
