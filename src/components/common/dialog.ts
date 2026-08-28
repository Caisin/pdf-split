import { open } from "@tauri-apps/plugin-dialog";

export async function pickPdfFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });

  return normalizeDialogSelection(selected);
}

export async function pickOutputDir() {
  const selected = await open({
    multiple: false,
    directory: true,
  });

  return normalizeDialogSelection(selected);
}

function normalizeDialogSelection(selected: string | string[] | null): string | null {
  if (selected == null) {
    return null;
  }

  return Array.isArray(selected) ? (selected[0] ?? null) : selected;
}

export function pathsLookSame(left: string, right: string) {
  return normalizeComparablePath(left) === normalizeComparablePath(right);
}

function normalizeComparablePath(value: string) {
  return value.trim().replace(/[\\/]+$/, "");
}
