type PickerFieldProps = {
  label: string;
  placeholder: string;
  value: string;
  buttonLabel: string;
  kind: "file" | "folder";
  onPick: () => void;
};

function FilePickerIcon({ kind }: { kind: "file" | "folder" }) {
  if (kind === "folder") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M3.5 7.5a2 2 0 0 1 2-2H9l2 2h7.5a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />
      </svg>
    );
  }

  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M7 4.5h7l3 3V19a1.5 1.5 0 0 1-1.5 1.5h-8A1.5 1.5 0 0 1 6 19V6A1.5 1.5 0 0 1 7.5 4.5z" />
      <path d="M14 4.5V8h3" />
    </svg>
  );
}

export function PickerField({
  label,
  placeholder,
  value,
  buttonLabel,
  kind,
  onPick,
}: PickerFieldProps) {
  return (
    <label className="field">
      <span>{label}</span>
      <div className="input-shell picker-shell">
        <input readOnly value={value} placeholder={placeholder} />
        <button
          className="picker-icon-button"
          type="button"
          aria-label={buttonLabel}
          title={buttonLabel}
          onClick={onPick}
        >
          <FilePickerIcon kind={kind} />
        </button>
      </div>
    </label>
  );
}
