import { useState } from "react";
import { useI18n } from "../hooks/useI18n";

interface JsonViewerProps {
  value: unknown;
  title?: string;
  defaultOpen?: boolean;
}

/** Collapsible, copyable pretty-printed JSON for debugging camera responses. */
export function JsonViewer({ value, title, defaultOpen = false }: JsonViewerProps) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  const text = JSON.stringify(value, null, 2);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };

  return (
    <details className="json-viewer" open={defaultOpen}>
      <summary>
        {title ?? t("common.rawJson")}
        <button type="button" className="btn btn-small btn-ghost" onClick={copy}>
          {copied ? t("common.copied") : t("common.copy")}
        </button>
      </summary>
      <pre>{text}</pre>
    </details>
  );
}
