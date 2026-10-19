// The page's icons: one 20-unit grid, one 1.6 stroke, round caps, drawn in currentColor.

import type { ReactNode } from "react";

function Icon({ children, size = 20, label }: { children: ReactNode; size?: number; label?: string }) {
  return (
    <svg
      viewBox="0 0 20 20"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      {children}
    </svg>
  );
}

type P = { size?: number; label?: string };

export const Reticle = (p: P) => (
  <Icon {...p}>
    <circle cx="10" cy="10" r="5.5" />
    <path d="M10 1.5v4M10 14.5v4M1.5 10h4M14.5 10h4" />
  </Icon>
);

export const Chevron = (p: P) => (
  <Icon {...p}>
    <path d="M5.5 8l4.5 4.5L14.5 8" />
  </Icon>
);

export const Pin = (p: P) => (
  <Icon {...p}>
    <path d="M10 18s5.5-5.2 5.5-9.5a5.5 5.5 0 1 0-11 0C4.5 12.8 10 18 10 18z" />
    <circle cx="10" cy="8.5" r="2" />
  </Icon>
);

export const Torch = (p: P) => (
  <Icon {...p}>
    <path d="M7 2.5h6l-1 5v8a2 2 0 0 1-4 0v-8z" />
    <path d="M8 7.5h4" />
  </Icon>
);

export const Upload = (p: P) => (
  <Icon {...p}>
    <path d="M10 13V3.5M6 7.5l4-4 4 4M3.5 13.5v2a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5v-2" />
  </Icon>
);

export const Check = (p: P) => (
  <Icon {...p}>
    <path d="M4 10.5l3.8 3.5L16 6" />
  </Icon>
);

export const Alert = (p: P) => (
  <Icon {...p}>
    <path d="M10 3l7.5 13h-15z" />
    <path d="M10 8.5v3.5M10 14.5v.01" />
  </Icon>
);

export const Survey = (p: P) => (
  <Icon {...p}>
    <circle cx="10" cy="10" r="7.5" />
    <path d="M2.5 10h15M10 2.5c2.2 2.2 3 4.7 3 7.5s-.8 5.3-3 7.5M10 2.5c-2.2 2.2-3 4.7-3 7.5s.8 5.3 3 7.5" />
  </Icon>
);
