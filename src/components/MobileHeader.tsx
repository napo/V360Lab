import { useCamera } from "../hooks/useCamera";
import { ConnectionBadge } from "./ConnectionBadge";

/** Compact top bar on phones: logo and connection state. */
export function MobileHeader() {
  const { connection } = useCamera();
  return (
    <header className="mobile-header">
      <img src="/brand/logo-dark-bg.png" alt="V360Lab" />
      <ConnectionBadge connection={connection} />
    </header>
  );
}
