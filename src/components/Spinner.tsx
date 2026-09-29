/** Animated loading ring in the brand gradient (the "loading GIF"). */
export function Spinner({ size = 28, label }: { size?: number; label?: string }) {
  return (
    <svg
      className="spinner"
      width={size}
      height={size}
      viewBox="0 0 50 50"
      role={label ? "img" : "presentation"}
      aria-label={label}
    >
      <defs>
        <linearGradient id="spinner-gradient" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#00b4ff" />
          <stop offset="100%" stopColor="#0072f5" />
        </linearGradient>
      </defs>
      <circle className="spinner-track" cx="25" cy="25" r="20" fill="none" strokeWidth="5" />
      <circle
        className="spinner-arc"
        cx="25"
        cy="25"
        r="20"
        fill="none"
        stroke="url(#spinner-gradient)"
        strokeWidth="5"
        strokeLinecap="round"
      />
    </svg>
  );
}
