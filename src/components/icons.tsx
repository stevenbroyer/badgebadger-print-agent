// Hand-rolled SVG icons. Skipping lucide-react / heroicons keeps the
// agent's bundle tiny — every byte counts in a binary you ship as a
// download. Each icon is a 24×24 stroke=2 viewbox; pass `size` to
// scale.

type IconProps = {
  size?: number;
  className?: string;
  strokeWidth?: number;
};

function base(size: number, strokeWidth: number, className?: string) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
    className,
  };
}

export function CheckIcon({ size = 16, className, strokeWidth = 2.4 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <polyline points="4 12.5 9.5 18 20 6.5" />
    </svg>
  );
}

export function XIcon({ size = 16, className, strokeWidth = 2.4 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <line x1="6" y1="6" x2="18" y2="18" />
      <line x1="6" y1="18" x2="18" y2="6" />
    </svg>
  );
}

export function CircleIcon({ size = 16, className, strokeWidth = 1.6 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <circle cx="12" cy="12" r="9" />
    </svg>
  );
}

export function PrinterIcon({ size = 16, className, strokeWidth = 1.7 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <polyline points="6 9 6 3 18 3 18 9" />
      <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
      <rect x="6" y="14" width="12" height="8" rx="1" />
    </svg>
  );
}

export function ZapIcon({ size = 16, className, strokeWidth = 2 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
    </svg>
  );
}

export function ClockIcon({ size = 16, className, strokeWidth = 1.7 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <circle cx="12" cy="12" r="9" />
      <polyline points="12 7 12 12 15.5 14" />
    </svg>
  );
}

export function CopyIcon({ size = 16, className, strokeWidth = 1.7 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

export function InfoIcon({ size = 16, className, strokeWidth = 1.8 }: IconProps) {
  return (
    <svg {...base(size, strokeWidth, className)}>
      <circle cx="12" cy="12" r="9" />
      <line x1="12" y1="11" x2="12" y2="17" />
      <line x1="12" y1="7.4" x2="12.01" y2="7.4" strokeWidth="2.4" />
    </svg>
  );
}
