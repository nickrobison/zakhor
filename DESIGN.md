# Zakhor Design System

## 1. Atmosphere & Identity

Zakhor is a quiet command center for persistent memory and knowledge graph operations. It should feel dense enough for technical review but spacious enough for fast scanning. The signature is neutral tonal layering: cards, sidebars, and result surfaces are separated with subtle background shifts and restrained borders instead of decorative effects.

## 2. Color

### Palette

| Role | Token | Light | Dark | Usage |
|------|-------|-------|------|-------|
| Surface/primary | --background | oklch(1 0 0) | oklch(0.145 0 0) | Page background |
| Surface/secondary | --card | oklch(1 0 0) | oklch(0.145 0 0) | Cards, sidebars, panels |
| Text/primary | --foreground | oklch(0.145 0 0) | oklch(0.985 0 0) | Headlines, primary body |
| Text/secondary | --muted-foreground | oklch(0.556 0 0) | oklch(0.708 0 0) | Captions, descriptions |
| Interactive/default | --primary | oklch(0.205 0 0) | oklch(0.985 0 0) | Primary buttons, active navigation |
| Interactive/on-primary | --primary-foreground | oklch(0.985 0 0) | oklch(0.205 0 0) | Primary button text |
| Surface/interactive | --accent | oklch(0.97 0 0) | oklch(0.269 0 0) | Hover surfaces, tabs |
| Text/on-interactive | --accent-foreground | oklch(0.205 0 0) | oklch(0.985 0 0) | Active tab text |
| Border/default | --border | oklch(0.922 0 0) | oklch(0.269 0 0) | Dividers, outlines |
| Input/default | --input | oklch(0.922 0 0) | oklch(0.269 0 0) | Inputs, controls |
| Focus | --ring | oklch(0.708 0 0) | oklch(0.439 0 0) | Focus rings |
| Status/error | --destructive | oklch(0.577 0.245 27.325) | oklch(0.396 0.141 25.723) | Errors and destructive states |

### Rules

- Use semantic Tailwind tokens (`bg-background`, `text-muted-foreground`, `border-border`) rather than raw colors.
- Accent is reserved for interactive hover and active states, not decoration.
- Do not introduce new colors unless they serve a distinct semantic role.

## 3. Typography

### Scale

| Level | Size | Weight | Line Height | Tracking | Usage |
|-------|------|--------|-------------|----------|-------|
| Page title | 36px / 2.25rem | 600 | 1.2 | -0.015em | Page headers |
| Card title | 22px / 1.375rem | 600 | 1.4 | 0 | Card headings |
| Body | 16px / 1rem | 400 | 1.6 | 0 | Default text |
| Body small | 14px / 0.875rem | 400 | 1.5 | 0 | Secondary text |
| Caption | 12px / 0.75rem | 500 | 1.4 | 0.02em | Metadata and labels |

### Font Stack

- Primary: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
- Mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace

### Rules

- Keep body text at 14px or larger.
- Headings use `tracking-tight` for page titles and card titles.
- Use semantic font classes (`text-muted-foreground`, `font-medium`) rather than arbitrary colors.

## 4. Spacing & Layout

### Base Unit

All spacing derives from a base of **4px**.

| Token | Value | Usage |
|-------|-------|-------|
| --space-2 | 8px | Inline groups, tight gaps |
| --space-3 | 12px | Compact form spacing |
| --space-4 | 16px | Standard card padding, input context |
| --space-5 | 20px | Comfortable section spacing |
| --space-6 | 24px | Card padding and page section gaps |
| --space-8 | 32px | Separated card groups |
| --space-10 | 40px | Page horizontal padding |
| --space-12 | 48px | Major section breaks |
| --space-16 | 64px | Page-level vertical rhythm |

### Grid

- Max content width: 1400px
- Column system: responsive grid with mobile-first single-column behavior and `md`/`lg`/`xl` expansion
- Breakpoints: sm 640px, md 768px, lg 1024px, xl 1280px, 2xl 1536px

### Rules

- Use Tailwind spacing units that map to 4px multiples.
- Page content lives in a fixed sidebar plus main content area; on narrow screens the sidebar remains fixed and content uses `p-8`.
- Avoid arbitrary spacing values outside the token set.

## 5. Components

### Page Shell

- **Structure**: fixed left navigation sidebar, main content area, full-height background
- **Variants**: default dashboard shell
- **Spacing**: sidebar `p-4`, main `p-8`, nav gap `gap-2`
- **States**: nav default, hover, active
- **Accessibility**: semantic `nav`, visible focus rings, text labels
- **Motion**: subtle background transition on nav hover

### Search Controls

- **Structure**: labeled search field, optional limit input, primary action button, tabs for result type filters
- **Variants**: all/decisions/entities/observations
- **Spacing**: input group `gap-2`, form stack `gap-4`
- **States**: default, focus, loading, empty, error
- **Accessibility**: labels for inputs, keyboard-accessible tabs, empty state copy
- **Motion**: no layout animation; tab content switches without motion

### Result Card

- **Structure**: bordered surface with title, metadata, snippet, and action affordance
- **Variants**: decision, entity, observation
- **Spacing**: card padding `p-4`, metadata gap `gap-2`, result list gap `gap-3`
- **States**: default, hover, active
- **Accessibility**: result title is focusable or linked, metadata is descriptive text
- **Motion**: hover background transition only

### Status Card

- **Structure**: card with title and badge value
- **Variants**: ok, unknown, unavailable
- **Spacing**: card padding `p-4`, badge margin top `mt-2`
- **States**: default, loading, error
- **Accessibility**: status text is readable by screen readers
- **Motion**: no animation

## 6. Motion & Interaction

### Timing

| Type | Duration | Easing | Usage |
|------|----------|--------|-------|
| Micro | 100-150ms | ease-out | Button press, tab focus |
| Standard | 200-300ms | ease-in-out | Navigation hover, card hover |
| Emphasis | 400-600ms | cubic-bezier(0.16, 1, 0.3, 1) | Not used in current pages |

### Rules

- Only animate `transform`, `opacity`, and background transitions.
- Respect `prefers-reduced-motion`; avoid non-essential animation.
- Every interactive element has hover and focus states.

## 7. Depth & Surface

### Strategy

**borders-only**

Cards, sidebars, and inputs are separated with 1px borders and semantic surface tokens. Do not use shadows or glass effects for page structure.

### Border Values

| Type | Value | Usage |
|------|-------|-------|
| Default | 1px solid var(--border) | Cards, dividers, inputs |
| Subtle | 1px solid color-mix(in oklch, var(--border) 70%, transparent) | Optional future soft separators |
