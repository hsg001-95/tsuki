---
version: alpha
name: Tsuki High-Craft Control Plane
description: Industrial, high-density telemetry and fail-closed safety control plane for Linux kernel reliability governed by the Anti-Slop Protocol.
colors:
  primary: "#00875A"
  secondary: "#0969DA"
  warning: "#D97706"
  approval: "#D9480F"
  critical: "#CF222E"
  surface: "#FFFFFF"
  surface-elevated: "#F6F8FA"
  surface-overlay: "#EAEEF2"
  background: "#F4F6F8"
  border: "#D0D7DE"
  border-muted: "#BAC3CF"
  border-strong: "#57606A"
  text: "#0F141C"
  text-secondary: "#57606A"
  text-muted: "#8C95A0"
typography:
  sans:
    fontFamily: "Space Grotesk, -apple-system, sans-serif"
  mono:
    fontFamily: "JetBrains Mono, monospace"
  display:
    fontFamily: "Space Grotesk, -apple-system, sans-serif"
    fontSize: "24px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  heading:
    fontFamily: "Space Grotesk, -apple-system, sans-serif"
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  subheading:
    fontFamily: "Space Grotesk, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.4
  body:
    fontFamily: "Space Grotesk, -apple-system, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  caption:
    fontFamily: "Space Grotesk, -apple-system, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0.02em"
  telemetry:
    fontFamily: "JetBrains Mono, monospace"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.4
  telemetry-lg:
    fontFamily: "JetBrains Mono, monospace"
    fontSize: "20px"
    fontWeight: 700
    lineHeight: 1.2
rounded:
  none: "0px"
  sm: "2px"
  base: "4px"
  md: "6px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  xxl: "32px"
---

## Overview

Tsuki is an authenticated, fail-closed operator control plane for Linux kernel reliability, continuous Pressure Stall Information (PSI) monitoring, and bounded cgroup v2 remediation. Designed under the Dashboard Anti-Slop Protocol, the interface discards generic AI SaaS conventions—such as floating card soups, bubbly pill badges, glowing gradient charts, and decorative emoji clutter—in favor of an engineered, high-craft instrumentation canvas. Visual hierarchy is established strictly through typography, whitespace calibration, and crisp 1px structural borders.

## Colors

The design enforces an uncompromising **90% neutral monochrome discipline**. Chromatic color is strictly forbidden as ambient decoration and is reserved exclusively for functional data semantics:

- Neutral Monochrome Canvas (90% Surface Area):
  - `background` (#F4F6F8): Non-reflective technical foundation.
  - `surface` (#FFFFFF): Unified canvas layer for structured telemetry sections.
  - `surface-elevated` (#F6F8FA): Inset inspector rows, diff containers, and timeline tracks.
  - `surface-overlay` (#EAEEF2): Split-pane modal triage drawers.
- Hairline Structural Borders:
  - `border` (#D0D7DE): Crisp 1px boundary lines dividing data cells.
  - `border-muted` (#BAC3CF): Subtle internal table dividers and axis markers.
  - `border-strong` (#57606A): High-contrast perimeter rules.
- High-Contrast Inks:
  - `text` (#0F141C): Primary high-contrast ink (WCAG AAA compliant).
  - `text-secondary` (#57606A): Technical labels and metric descriptions.
  - `text-muted` (#8C95A0): Secondary units and inactive indicators.
- Semantic State Reserve (Strictly Functional):
  - `primary` (#00875A, Forest Emerald): Nominal health, valid configuration, single primary action.
  - `secondary` (#0969DA, Avionics Blue): Read-only observation, verified checkpoint, chart telemetry line.
  - `warning` (#D97706, Industrial Amber): PSI pressure escalation, secondary chart telemetry line.
  - `approval` (#D9480F, Safety Orange): Staged intent awaiting authenticated human-in-the-loop authorization.
  - `critical` (#CF222E, Signal Vermilion): Active kernel freeze, Emergency Panic Latch active, rollback failure.

## Themes

Tsuki provides two high-craft themes adhering to the 90% monochrome discipline:

| Theme | Substrate | Text | Structural Border | Primary Accent | Critical State |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Industrial Light (Canonical) | #F4F6F8 | #0F141C | #D0D7DE | #00875A | #CF222E |
| Industrial Dark (Alternate) | #090A0F | #E6EDF3 | #27272A | #00E5A3 | #FF2A42 |

## Typography

Default fonts (Inter, Roboto, system web fallbacks) are replaced with an intentional high-craft pairing:

- Interface Chrome & Narrative Labels: Space Grotesk provides structured, mechanical authority for titles, section headers, form fields, and causal explanations.
- Telemetry & Computational Values: JetBrains Mono governs all numerical data, timestamps, memory counters, cgroup v2 paths, Unix socket paths, and journal diffs.
- Mandatory Tabular Numbers: All KPI metric displays, tables, timestamps, percentages, and latencies strictly enforce `font-variant-numeric: tabular-nums` to ensure vertical column alignment without layout jitter.
- Epistemic Integrity: Verified low-level kernel metrics (/proc/pressure/*, /proc/vmstat) are always rendered in monospace telemetry styles, while RAG-derived documentation citations and causal analysis are rendered in structured sans-serif prose.

## Layout

The layout replaces bloated SaaS layouts with high-density, scannable data architecture:

- Unified Canvas vs Floating Card Soup: Floating disconnected cards over grey backdrops are banned. Data sections sit within a continuous structured canvas divided by crisp 1px solid hairline borders.
- Compact KPI Summary Strip: Top telemetry is displayed in a contiguous, tightly padded strip (`p-3` / `p-4`) displaying real-time values, direct textual deltas, and minimal linear tracks.
- Tight Padding & Spacing: Bloated padding (`p-8`, 32px+) is eliminated in favor of calibrated 8px–16px structural spacing, maximizing situational awareness during incidents.
- Sticky Table Headers: Telemetry ledgers, cgroup trees, and rollback journals enforce sticky table headers for uninterrupted scanning during high-stress triage.
- Split-Pane Triage Drawer: Candidate remediation reviews slide out from the right viewport boundary without occluding live background pressure time-series.

## Elevation & Depth

Depth is established strictly through 1px structural strokes and flat tonal stepping:

- Zero Ambient Drop Shadows: Soft multi-stop box shadows (`0 10px 15px -3px rgba(0,0,0,0.1)`), glowing card edges, and frosted glass blurs are completely eliminated.
- Flat Tonal Stepping: Hierarchy is conveyed by contrasting `surface` (#FFFFFF) against inset `surface-elevated` (#F6F8FA) containers.
- Hairline Inset Accents: Elevated panels and buttons use an ultra-fine 1px interior highlight (`box-shadow: inset 0 1px 0 0 rgba(0, 0, 0, 0.04)`) to define physical edges cleanly.
- Alert Demarcation: Critical and warning states use crisp 1px or 2px solid perimeter strokes, never diffuse ambient glows.

## Shapes

Shapes enforce rigid architectural discipline:

- Tight Border Radii: All structural panels, data cells, tables, inputs, and buttons enforce tight corner radii between `2px` and `4px`, with a maximum ceiling of `6px` on modals.
- Banned Pill Badges: Rounded pill badges (`border-radius: 9999px`) are strictly prohibited. Status indicators and mode tags are rendered as rectangular chips with `2px` corners and 1px inset borders.
- Linear Dividers: Layout regions are demarcated by 1px solid hairline rules.

## Components

Components are engineered for sub-15-second incident triage:

- KPI Telemetry Strip: Contiguous cell array pairing live kernel counters with direct status text and subtle 4px linear progress tracks.
- Precision Telemetry Charts: Time-series graphs utilize sharp 1.5px stroke lines, direct axis data labels, and zero multi-stop gradient area fills.
- Split-Pane Triage Drawer: Monospace parameter diff viewer contrasting baseline values (`max`) against proposed limits (`1.02 GB`), accompanied by an automated 6-point safety check matrix.
- High-Density Data Tables: Strict tabular figures, compact row heights, sticky headers, and monospace alignment for cgroup trees and rollback journals.
- Mode-Aware Control Bar: Persistent status header displaying active host, config validity, ONNX detector latency, and operating mode badge.

## Do's and Don'ts

- Do enforce `font-variant-numeric: tabular-nums` on all streaming counters, latencies, and kernel PSI metrics.
- Do use crisp 1px solid hairline borders to divide data sections on a unified canvas.
- Do maintain live background PSI chart rendering when the triage drawer is open.
- Do keep 90% of the visual interface neutral monochrome, reserving color exclusively for semantic data states.
- Don't use floating card soups with rounded-2xl corners and soft drop shadows.
- Don't use glowing gradient charts, neon area fills, or decorative ambient blurs.
- Don't use emojis or generic decorative icon spam inside metric cards, headers, or table rows.
- Don't use default fonts like Inter or Roboto for metrics and technical data.
- Don't use rounded pill badges (`border-radius: 9999px`) or bloated SaaS padding (`p-8`).

## Forbidden Dashboard Anti-Patterns

1. **Floating Card Soup**: Banned `rounded-2xl` cards with soft diffuse box-shadows floating over `#F3F4F6` grey backdrops. Replaced with a unified canvas divided by crisp `1px solid` hairline borders.
2. **Glowing Gradient Charts**: Banned multi-stop neon, purple, or cyan area fills under line charts. Replaced with sharp `1.5px` stroke lines, subtle monochrome fills, and direct numerical data labels.
3. **Emoji & Decorative Icon Spam**: Banned emojis (e.g. 🌙, ⚡, ⚠️, 🛡️, 📜) and decorative icons inside KPI headers, badges, and table rows. Typography and structural whitespace govern hierarchy.
4. **Default Font Laziness**: Banned uncalibrated default fonts (`Inter`, `Roboto`, system fallbacks). Replaced with the intentional high-craft pairing of `Space Grotesk` (chrome & narratives) and `JetBrains Mono` with `tabular-nums` (telemetry & metrics).
5. **Bloated SaaS Spacing**: Banned excessive whitespace (`p-8`, `gap-8`) that reduces data density. Replaced with tight, scannable spacing (`p-3` / `p-4`), compact KPI summary strips, and sticky table headers.
