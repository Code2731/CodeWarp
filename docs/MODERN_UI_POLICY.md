# CodeWarp Modern UI Policy (v1)

Last updated: 2026-05-23

## 1) Goal
- Make the UI feel modern, intentional, and consistent.
- Reduce "assembled" feeling by enforcing a token-first system.
- Keep design and code in sync with Figma variables and semantic tokens.

## 2) Research Basis
- Figma Variables and token workflows:
  - https://help.figma.com/hc/en-us/articles/15339657135383-Guide-to-variables-in-Figma
  - https://help.figma.com/hc/en-us/articles/15871097384471-The-difference-between-variables-and-styles
  - https://help.figma.com/hc/en-us/articles/15343816063383-Modes-for-variables
- Fluent token and spacing system:
  - https://fluent2.microsoft.design/design-tokens
  - https://fluent2.microsoft.design/layout
- Visual hierarchy and readability:
  - https://developer.apple.com/design/human-interface-guidelines/?authuser=0
  - https://developer.apple.com/design/human-interface-guidelines/typography?changes=_10_7
- Accessibility gates:
  - https://www.w3.org/TR/WCAG22/
  - https://www.w3.org/WAI/WCAG21/Techniques/css/C39.html

## 3) Non-Negotiable Rules

### 3.1 Token-first
- Do not hardcode spacing, radius, or color values in feature code.
- Use shared UI tokens from `src/view/ui.rs`.
- New visual values must be introduced as tokens first, then consumed by views.

### 3.2 Three token layers
- Primitive: raw values (`space-xs`, `radius-sm`, base palette).
- Semantic: role values (`surface/subtle`, `text/muted`, `state/danger`).
- Component: mapped usage (`context-item-bg`, `context-item-border`).

### 3.3 Hierarchy
- Use a maximum of 3 text emphasis levels in one panel.
- Primary info: semibold body.
- Secondary info: micro label.
- Metrics/system values: mono micro.

### 3.4 Layout rhythm
- Use one spacing ramp only.
- Keep panel internals on a consistent vertical rhythm.
- Avoid mixed arbitrary paddings in the same component.

### 3.5 Accessibility gates (AA minimum)
- Text contrast: 4.5:1 for normal text, 3:1 for large text.
- Non-text indicators should not rely on color only.
- Motion must support reduced-motion preference when animation is introduced.
- Keyboard focus must remain visible on interactive controls.

## 4) Figma -> Code Operating Model
- Figma variables are source of truth for design tokens.
- Prefer semantic names (`surface/subtle`, `text/secondary`) over visual names (`gray-500`).
- Maintain 1:1 mapping between Figma token names and code token names.
- Theme modes (light/dark) must be represented as token modes, not per-component overrides.

## 5) Code Review Checklist
- No new hardcoded spacing/radius/color in view logic.
- Component uses shared style helper where available.
- Hierarchy is clear within one scan (title -> content -> metadata).
- Empty/loading/error states are visually differentiated and readable.
- Contrast and focus states are verified before merge.

## 6) Current Round Scope (Applied)
- Introduced shared spacing and context panel tokens in `src/view/ui.rs`.
- Added a reusable `context_item_style` style helper.
- Updated sidebar context panel in `src/view.rs` to consume tokens instead of local magic numbers.

