const BLADES = 6;

/**
 * A lens diaphragm drawn with six blades. Each blade is a half-plane whose
 * edge is pushed away from the center when open, so together they leave a
 * hexagonal aperture; CSS animates the blades (see `.aperture` styles).
 */
export function ApertureIcon({ open }: { open: boolean }) {
  return (
    <svg className={`aperture ${open ? "open" : ""}`} viewBox="-50 -50 100 100" aria-hidden="true">
      <defs>
        <clipPath id="aperture-clip">
          <circle r="48" />
        </clipPath>
        <radialGradient id="aperture-glass" cx="35%" cy="35%" r="75%">
          <stop offset="0" stopColor="#7fe0ff" />
          <stop offset="0.45" stopColor="#0072f5" />
          <stop offset="1" stopColor="#030912" />
        </radialGradient>
      </defs>
      <g clipPath="url(#aperture-clip)">
        <circle className="aperture-glass" r="48" fill="url(#aperture-glass)" />
        {Array.from({ length: BLADES }, (_, i) => (
          <g key={i} className={`aperture-blade ${i % 2 ? "alt" : ""}`} style={{ ["--angle" as string]: `${(360 / BLADES) * i}deg` }}>
            <rect x="-100" y="0" width="200" height="100" />
          </g>
        ))}
      </g>
      <circle className="aperture-rim" r="48" />
    </svg>
  );
}
