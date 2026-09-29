import type { ReactNode } from "react";
import { EMPTY } from "../utils/format";

export interface KeyValue {
  label: string;
  value: ReactNode;
  /** Render the value in a monospace font. */
  mono?: boolean;
}

export function KeyValueList({ items }: { items: KeyValue[] }) {
  return (
    <dl className="kv-list">
      {items.map(({ label, value, mono }) => (
        <div className="kv-row" key={label}>
          <dt>{label}</dt>
          <dd className={mono ? "mono" : undefined}>
            {value === null || value === undefined || value === "" ? EMPTY : value}
          </dd>
        </div>
      ))}
    </dl>
  );
}
