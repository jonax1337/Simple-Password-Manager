// Shared Tailwind class recipes. Used by primitives and call sites that need
// the same look (icon buttons, eyebrow labels, tiles) without re-typing the
// 60-character class strings every time.
import { tv, type VariantProps } from "tailwind-variants";

// ─── Typography ─────────────────────────────────────────────────────────────

// Tiny uppercase section/field label — used 20+ times across the app.
export const eyebrow =
  "text-2xs font-semibold uppercase tracking-wider text-muted-foreground";

// Smaller eyebrow used inside FieldRow above the value.
export const fieldLabel =
  "text-2xs font-medium uppercase tracking-wider text-muted-foreground/80";

// "Metadata" lines (timestamps, hint copy, footer chrome).
export const metaText = "text-xs text-muted-foreground";

// Standard body row text in lists and field values.
export const bodyText = "text-sm";

// ─── Focus ring ─────────────────────────────────────────────────────────────

// Single focus-ring pattern applied to all interactive primitives. WCAG 2.4.11
// requires ≥2px thick and ≥3:1 contrast against both element and adjacent
// background. The ring-offset solves the contrast-against-adjacent half.
export const focusRing =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

// ─── Icon button ────────────────────────────────────────────────────────────

// Square icon-only button. Default 36×36 (matches form control height), with
// a compact 28×28 for inline row actions, and an optional `softReveal` mode
// that dims to 60% until the parent row is hovered (WCAG 2.2 SC 3.2.7).
export const iconButton = tv({
  base: `inline-flex items-center justify-center rounded-md transition-colors shrink-0 disabled:opacity-50 disabled:pointer-events-none ${focusRing}`,
  variants: {
    size: {
      sm: "size-7 [&_svg]:size-3.5",
      md: "size-9 [&_svg]:size-4",
    },
    tone: {
      ghost: "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
      muted: "text-muted-foreground hover:bg-muted hover:text-foreground",
      // Soft-tinted "muted" variant for action rows inside cards (no full bg).
      soft: "text-muted-foreground hover:bg-foreground/10 hover:text-foreground",
      primary:
        "bg-primary text-primary-foreground hover:bg-primary/90 shadow-xs",
      "primary-soft":
        "bg-primary/10 text-primary hover:bg-primary/15",
      destructive:
        "text-destructive hover:bg-destructive/10",
    },
    // "Soft reveal" follows WCAG 2.2 SC 3.2.7 — actions stay visible at
    // reduced opacity for discoverability and brighten on hover/focus.
    // 60% is the lowest opacity that keeps a non-decorative control passable
    // by SC 1.4.11 contrast-against-non-text.
    softReveal: {
      true: "opacity-60 group-hover/row:opacity-100 group-hover/field:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100 transition-opacity",
      false: "",
    },
  },
  defaultVariants: {
    size: "md",
    tone: "ghost",
    softReveal: false,
  },
});
export type IconButtonVariants = VariantProps<typeof iconButton>;

// ─── Surfaces ───────────────────────────────────────────────────────────────

// Standard card surface. Used by stat tiles, callouts, and the field-row
// container. `interactive` adds a hover background for clickable surfaces.
export const tile = tv({
  base: "rounded-xl border bg-card shadow-xs",
  variants: {
    padding: {
      sm: "p-3",
      md: "p-4",
      lg: "px-4 py-3",
    },
    interactive: {
      true: "hover:bg-accent/30 transition-colors",
      false: "",
    },
  },
  defaultVariants: { padding: "md", interactive: false },
});
export type TileVariants = VariantProps<typeof tile>;

// Hairline-divided card (login details, additional fields).
export const groupedCard =
  "rounded-lg border bg-card shadow-xs overflow-hidden";

// ─── Navigation rows ────────────────────────────────────────────────────────

// Used by Sidebar primary nav and SettingsDialog left rail.
export const navRow = tv({
  base: "w-full flex items-center gap-2.5 rounded-md px-2.5 text-sm font-medium transition-colors",
  variants: {
    active: {
      true: "bg-selected text-selected-foreground",
      false: "text-foreground/80 hover:bg-accent/60 hover:text-foreground",
    },
    size: {
      sm: "h-8",
      md: "h-9",
    },
  },
  defaultVariants: { active: false, size: "sm" },
});
