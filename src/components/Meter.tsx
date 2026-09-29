interface MeterProps {
  /** Value between 0 and 1. */
  fraction: number | null;
  label?: string;
  tone?: "default" | "warning" | "danger";
}

export function Meter({ fraction, label, tone = "default" }: MeterProps) {
  const percent = fraction == null ? 0 : Math.round(Math.min(1, Math.max(0, fraction)) * 100);
  return (
    <div className="meter" aria-label={label}>
      <div className={`meter-fill meter-${tone}`} style={{ width: `${percent}%` }} />
    </div>
  );
}
