/**
 * pain ai — Design System Tokens
 * Source of truth: DESIGN.md §2–§5.
 * 
 * NOTE: Hex codes are permitted ONLY in this file.
 * All application code and styles must reference CSS variables or these tokens.
 */

export const colors = {
  // Brand & Accent
  primary: "#cc785c",
  primaryActive: "#a9583e",
  primaryDisabled: "#e6dfd8",
  accentTeal: "#5db8a6",
  accentAmber: "#e8a55a",

  // Surface
  canvas: "#faf9f5",
  surfaceSoft: "#f5f0e8",
  surfaceCard: "#efe9de",
  surfaceCreamStrong: "#e8e0d2",
  surfaceDark: "#181715",
  surfaceDarkElevated: "#252320",
  surfaceDarkSoft: "#1f1e1b",
  hairline: "#e6dfd8",
  hairlineSoft: "#ebe6df",

  // Text
  ink: "#141413",
  bodyStrong: "#252523",
  body: "#3d3d3a",
  muted: "#6c6a64",
  mutedSoft: "#8e8b82",
  onPrimary: "#ffffff",
  onDark: "#faf9f5",
  onDarkSoft: "#a09d96",

  // Semantic
  success: "#5db872",
  warning: "#d4a017",
  error: "#c64545",
} as const;

export const spacing = {
  xxs: "4px",
  xs: "8px",
  sm: "12px",
  md: "16px",
  lg: "24px",
  xl: "32px",
  xxl: "48px",
  section: "96px",
} as const;

export const rounded = {
  xs: "4px",
  sm: "6px",
  md: "8px",
  lg: "12px",
  xl: "16px",
  pill: "9999px",
  full: "50%",
} as const;

export const typography = {
  family: {
    display: 'Cormorant Garamond, Tiempos Headline, Garamond, "Times New Roman", serif',
    body: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    code: 'JetBrains Mono, monospace',
  },
  displayXl: {
    fontSize: "64px",
    fontWeight: 400,
    lineHeight: 1.05,
    letterSpacing: "-1.5px",
  },
  displayLg: {
    fontSize: "48px",
    fontWeight: 400,
    lineHeight: 1.1,
    letterSpacing: "-1px",
  },
  displayMd: {
    fontSize: "36px",
    fontWeight: 400,
    lineHeight: 1.15,
    letterSpacing: "-0.5px",
  },
  displaySm: {
    fontSize: "28px",
    fontWeight: 400,
    lineHeight: 1.2,
    letterSpacing: "-0.3px",
  },
  titleLg: {
    fontSize: "22px",
    fontWeight: 500,
    lineHeight: 1.3,
    letterSpacing: "0px",
  },
  titleMd: {
    fontSize: "18px",
    fontWeight: 500,
    lineHeight: 1.4,
    letterSpacing: "0px",
  },
  titleSm: {
    fontSize: "16px",
    fontWeight: 500,
    lineHeight: 1.4,
    letterSpacing: "0px",
  },
  bodyMd: {
    fontSize: "16px",
    fontWeight: 400,
    lineHeight: 1.55,
    letterSpacing: "0px",
  },
  bodySm: {
    fontSize: "14px",
    fontWeight: 400,
    lineHeight: 1.55,
    letterSpacing: "0px",
  },
  caption: {
    fontSize: "13px",
    fontWeight: 500,
    lineHeight: 1.4,
    letterSpacing: "0px",
  },
  captionUppercase: {
    fontSize: "12px",
    fontWeight: 500,
    lineHeight: 1.4,
    letterSpacing: "1.5px",
  },
  code: {
    fontSize: "14px",
    fontWeight: 400,
    lineHeight: 1.6,
    letterSpacing: "0px",
  },
  button: {
    fontSize: "14px",
    fontWeight: 500,
    lineHeight: 1.0,
    letterSpacing: "0px",
  },
  navLink: {
    fontSize: "14px",
    fontWeight: 500,
    lineHeight: 1.4,
    letterSpacing: "0px",
  },
} as const;

export type ColorToken = keyof typeof colors;
export type SpacingToken = keyof typeof spacing;
export type RoundedToken = keyof typeof rounded;
export type TypographyToken = keyof typeof typography;
