---
name: dashboard-anti-slop
description: Audits and rewrites dashboard DESIGN.md files to replace generic AI SaaS templates with crisp, high-craft product UI.
---

# Dashboard Anti-Slop Design Protocol

When asked to audit or rewrite a `DESIGN.md` file for a dashboard:

## 1. Strip AI Dashboard Clichés (NEVER USE)
- **No Floating Card Soup:** Ban `rounded-2xl` cards with soft `box-shadow` floating over `#F3F4F6` grey backgrounds. Use a unified canvas divided by crisp `1px solid` hairline borders instead.
- **No Glowing Gradient Charts:** Ban multi-stop neon/purple area fills under line charts. Use sharp `1.5px` stroke lines, subtle monochrome fills, and direct data labels.
- **No Emoji or Generic Lucide Icon Spam:** Ban decorative icons inside every KPI header or table row. Let typography and whitespace create hierarchy.
- **No Default Fonts:** Replace `Inter` or `Roboto` with an intentional UI pairing (e.g., `Geist` or `Plus Jakarta Sans` for labels, paired with `JetBrains Mono` or `Geist Mono` for metrics).

## 2. Enforce High-Craft Dashboard Tokens
- **Surface & Borders:** Use tight border radii (`4px` to `6px` max) and subtle `1px` structural borders (`#E5E7EB` in light mode / `#27272A` in dark mode) instead of shadows.
- **Typography & Numbers:** Mandate `font-variant-numeric: tabular-nums` on all KPIs, tables, timestamps, and percentages so numbers align vertically.
- **Color Discipline:** Enforce a 90% neutral monochrome UI (zinc/slate/neutral grays). Reserve color strictly for semantic data states (success, warning, critical) and a single crisp primary action color.
- **Layout & Density:** Replace bloated padding (`p-8`) with tight, scannable spacing (`p-3` or `p-4`), sticky table headers, and compact KPI summary strips.

## 3. Output Action
Read the user's `DESIGN.md`, replace any generic SaaS boilerplate with the rules above, append a `## Forbidden Dashboard Anti-Patterns` section, and save the updated file directly.
