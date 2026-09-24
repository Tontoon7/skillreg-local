# SkillReg Local — Existing Design System

## Purpose

This document records the visual system already implemented in SkillReg Local. It is an
inventory, not a redesign proposal. New employee-facing screens must reuse these foundations
until a separate visual-direction decision is approved.

Authoritative source: `src/styles/globals.css` and the components under `src/components/ui/`.

## Character

The existing product uses a dark, compact, industrial control-panel aesthetic:

- near-black navy background;
- layered blue surfaces;
- amber as the main action and focus color;
- cyan as a secondary active signal;
- small LED-like status indicators;
- inset and raised panels;
- subtle brushed-metal texture;
- restrained glow for state and focus;
- Chakra Petch for display and control labels;
- system sans-serif for body copy.

The employee redesign should make the information simpler without making the product generic
or removing this recognizable character.

## Color tokens

| Token | Current value | Intended use |
|---|---|---|
| `background` | `#060b16` | application canvas |
| `foreground` | `#e4e9f0` | default text |
| `card` | `#0c1326` | cards and inset panels |
| `card-foreground` | `#edf1f7` | high-emphasis card text |
| `popover` | `#0c1326` | popovers |
| `popover-foreground` | `#edf1f7` | popover text |
| `primary` | `#e5a033` | primary action, focus, active navigation |
| `primary-foreground` | `#0a0600` | text on primary |
| `secondary` | `#162240` | secondary controls and hover |
| `secondary-foreground` | `#a8b5c8` | secondary text |
| `muted` | `#131d34` | subdued fills |
| `muted-foreground` | `#7e90a8` | supporting text |
| `accent` | `#22d3ee` | secondary positive/active signal |
| `accent-foreground` | `#060b16` | text on accent |
| `destructive` | `#ef4444` | destructive/error state |
| `border` | `#1e2f50` | structural borders |
| `input` | `#0e1628` | form controls |
| `ring` | `#e5a033` | keyboard focus |
| `surface` | `#101b30` | code and nested surfaces |
| `sidebar` | `#0a1020` | navigation background |

Do not introduce additional semantic colors until the current tokens prove insufficient. Use
text and iconography alongside color for every status.

## Glow tokens

| Token | Current value |
|---|---|
| `glow-amber` | `rgba(229, 160, 51, 0.5)` |
| `glow-cyan` | `rgba(34, 211, 238, 0.4)` |
| `glow-red` | `rgba(239, 68, 68, 0.45)` |

Glow is reserved for focus, active navigation, primary actions, and small status indicators.
It must not be applied to every card.

## Typography

### Body

```css
-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif
```

Use for paragraphs, descriptions, forms, and data.

### Display and controls

```css
"Chakra Petch", sans-serif
```

Currently used for:

- headings;
- buttons;
- card titles;
- navigation section labels;
- product title.

Headings use `0.02em` tracking. Small labels use wider uppercase tracking.

## Radius

Base token: `0.625rem`.

Current patterns:

- controls: `rounded-md`;
- nested controls: `rounded-lg`;
- cards: `rounded-xl`;
- status LEDs and badges: full circle/pill.

## Surfaces

### Inset panel

`.panel-inset`:

- card background;
- one-pixel border;
- dark inset shadows;
- subtle light top edge.

Use for state summaries, configuration, and content that belongs inside the application
frame.

### Raised panel

`.panel-raised`:

- blue vertical gradient;
- brighter border;
- external shadow;
- subtle internal highlights.

Use for an actionable module or elevated control, not every section.

### Brushed metal

`.brushed-metal` combines a subtle horizontal texture with a dark navy gradient.

Use only for persistent chrome such as the titlebar and sidebar.

## Borders

`.channel-border-b` and `.channel-border-r` combine a border with a faint highlight. They are
used to separate persistent application regions.

## Status indicators

Existing LED variants:

- amber: active or selected;
- cyan: ready/connected when a secondary signal is needed;
- red: error/destructive;
- off: inactive or unavailable.

Every LED needs adjacent text. A glowing dot alone is not an accessible status.

## Components

### Button

- Chakra Petch;
- medium weight;
- minimum height 32–40 px depending on size;
- visible `focus-visible` ring;
- variants: default, destructive, outline, secondary, ghost, link.

The default variant is the single primary action. Avoid two default buttons in one panel.

### Card

- inset panel;
- 24 px vertical and horizontal content rhythm;
- optional hover glow currently exists.

For the employee dashboard, non-clickable cards should not glow on hover. Use hover feedback
only when the entire card is interactive.

### Badge

- compact pill;
- border and text;
- optional restrained glow.

Badges describe trust or compact status. Do not use a badge in place of an actionable error
message.

### Input and select

Use the existing input, label, and select primitives. Keep technical values out of the normal
employee flow.

### Titlebar

- macOS: native traffic lights with overlay titlebar;
- Windows/Linux: custom controls;
- brushed-metal background;
- height around 36–38 px.

Do not place product actions in the draggable region.

### Sidebar

- fixed 224 px width today;
- brushed-metal background;
- channel border;
- active item combines amber LED, icon, text, and left border.

The employee navigation reduces the number of primary items but retains this treatment.

## Spacing and layout

Observed Tailwind rhythm:

- page padding: `p-6` (24 px);
- primary section gaps: `gap-6` (24 px);
- card inner gaps: 8–16 px;
- sidebar item padding: 12 px horizontal, 8 px vertical;
- form width: approximately 384–448 px;
- desktop app uses a full-height shell with independent main-content scrolling.

New pages should use the same 4/8/12/16/24 px rhythm.

## Motion

Existing motion:

- color and shadow transitions on controls;
- `pulse-glow` at 2 seconds;
- `flicker-in` at 300 ms;
- spinner rotation from Tailwind.

Rules:

- motion communicates state changes, never decoration alone;
- respect `prefers-reduced-motion`;
- do not pulse a healthy state continuously;
- use spinner/skeleton dimensions that prevent layout shift;
- keep transitions interruptible and under 300 ms for direct manipulation.

## Accessibility

- preserve the existing focus-visible ring;
- all controls must be reachable and operable by keyboard;
- status must combine text, icon, and color;
- minimum interactive target is 36 px in the desktop shell;
- destructive actions require text and confirmation;
- support 200% text scaling without clipping;
- avoid muted text for required instructions;
- ensure titlebar drag regions never intercept buttons;
- provide reduced-motion behavior for glow/pulse effects.

## Employee-screen hierarchy

Use this visual priority:

1. global health;
2. automatic-update control;
3. required actions;
4. installed skills;
5. secondary recommendations.

The organization switcher is secondary chrome when multiple organizations exist, not the main
dashboard content.

## Responsive window behavior

Primary QA sizes:

- 1024×700 standard;
- current minimum window size from Tauri configuration;
- narrow desktop window with sidebar preserved or collapsed according to the shape brief.

Avoid mobile-web patterns such as bottom navigation unless a separate desktop-window decision
is made.

## Light mode

The current extracted tokens define only the dark visual system. Existing UI references a
theme setting, but a complete light token set is not present in `globals.css`.

Until light tokens are intentionally defined and verified:

- do not claim full light-mode support;
- do not synthesize colors ad hoc in individual components;
- track light-mode completion as a separate design-system task.

## Anti-patterns

- new purple/indigo brand accents that conflict with amber/cyan;
- gradients on every surface;
- excessive card grids;
- all-caps body copy;
- glow without semantic purpose;
- hiding focus outlines;
- icon-only error states;
- technical labels in primary employee flows;
- hover affordance on non-interactive content.
