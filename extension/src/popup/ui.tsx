// Minimal shadcn-style primitives mirrored from the desktop app's UI lib,
// trimmed to what the popup actually needs. Same class-token vocabulary
// (`bg-background`, `text-foreground`, etc.) so the popup and the app feel
// like one product.

import * as React from "react";

export function cn(...classes: (string | false | null | undefined)[]) {
  return classes.filter(Boolean).join(" ");
}

// ---------- Button ----------

type ButtonVariant = "default" | "outline" | "ghost" | "secondary" | "destructive";
type ButtonSize = "default" | "sm" | "icon";

const buttonBase =
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-lg text-sm font-medium transition-all duration-150 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:pointer-events-none disabled:opacity-50 active:scale-[0.98]";

const buttonVariants: Record<ButtonVariant, string> = {
  default:
    "bg-gradient-to-b from-primary/95 to-primary text-primary-foreground shadow-sm shadow-primary/30 hover:from-primary hover:to-primary/95 hover:shadow-md hover:shadow-primary/40",
  outline:
    "border border-input bg-background/60 backdrop-blur-sm hover:bg-accent hover:text-accent-foreground hover:border-primary/30",
  ghost: "hover:bg-accent hover:text-accent-foreground",
  secondary:
    "bg-secondary text-secondary-foreground hover:bg-secondary/80",
  destructive:
    "bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90",
};

const buttonSizes: Record<ButtonSize, string> = {
  default: "h-9 px-3.5 py-2",
  sm: "h-7 px-2.5 text-xs",
  icon: "h-8 w-8",
};

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    { className, variant = "default", size = "default", type = "button", ...props },
    ref,
  ) => (
    <button
      ref={ref}
      type={type}
      className={cn(buttonBase, buttonVariants[variant], buttonSizes[size], className)}
      {...props}
    />
  ),
);
Button.displayName = "Button";

// ---------- Card ----------

export function Card(props: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...props}
      className={cn(
        "rounded-lg border border-border/80 bg-card/95 backdrop-blur-sm text-card-foreground shadow-sm shadow-foreground/[0.02]",
        props.className,
      )}
    />
  );
}

// ---------- Input ----------

export const Input = React.forwardRef<
  HTMLInputElement,
  React.InputHTMLAttributes<HTMLInputElement>
>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={cn(
      "flex h-9 w-full rounded-lg border border-input bg-background/70 px-3 py-1 text-sm transition-all duration-150",
      "placeholder:text-muted-foreground/70",
      "focus-visible:outline-hidden focus-visible:border-primary/50 focus-visible:ring-2 focus-visible:ring-primary/20",
      "hover:border-input/80",
      "disabled:cursor-not-allowed disabled:opacity-50",
      className,
    )}
    {...props}
  />
));
Input.displayName = "Input";

// ---------- Label ----------

export function Label(
  props: React.LabelHTMLAttributes<HTMLLabelElement>,
) {
  return (
    <label
      {...props}
      className={cn(
        "text-xs font-medium leading-none text-muted-foreground",
        props.className,
      )}
    />
  );
}

// ---------- Separator ----------

export function Separator(props: { className?: string }) {
  return (
    <div
      className={cn("h-px w-full bg-border", props.className)}
      role="separator"
    />
  );
}

// ---------- Letter Badge ----------
// Colored avatar shown to the left of each entry — like 1Password's icons.
// Hash the title to a stable hue so each entry keeps the same color forever.

// Soft duo-tone gradients per badge — keeps the popup feeling premium and
// helps the eye pick out entries at a glance, the way 1Password's coloured
// avatars do.
const PALETTE = [
  ["from-indigo-500/25 to-violet-500/15", "text-indigo-700 dark:text-indigo-200", "ring-indigo-500/20"],
  ["from-rose-500/25 to-pink-500/15", "text-rose-700 dark:text-rose-200", "ring-rose-500/20"],
  ["from-emerald-500/25 to-teal-500/15", "text-emerald-700 dark:text-emerald-200", "ring-emerald-500/20"],
  ["from-amber-500/25 to-orange-500/15", "text-amber-700 dark:text-amber-200", "ring-amber-500/20"],
  ["from-sky-500/25 to-cyan-500/15", "text-sky-700 dark:text-sky-200", "ring-sky-500/20"],
  ["from-violet-500/25 to-fuchsia-500/15", "text-violet-700 dark:text-violet-200", "ring-violet-500/20"],
  ["from-pink-500/25 to-rose-500/15", "text-pink-700 dark:text-pink-200", "ring-pink-500/20"],
  ["from-teal-500/25 to-emerald-500/15", "text-teal-700 dark:text-teal-200", "ring-teal-500/20"],
];

function hashTitle(title: string): number {
  let h = 0;
  for (let i = 0; i < title.length; i++) {
    h = ((h << 5) - h + title.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

export function LetterBadge(props: { title: string; size?: "sm" | "md" }) {
  const initial = (props.title.trim()[0] ?? "?").toUpperCase();
  const [gradient, fg, ring] = PALETTE[hashTitle(props.title) % PALETTE.length];
  const dimensions =
    props.size === "md" ? "h-9 w-9 text-sm" : "h-7 w-7 text-xs";
  return (
    <div
      className={cn(
        "relative flex shrink-0 items-center justify-center rounded-lg font-semibold",
        "bg-gradient-to-br",
        gradient,
        fg,
        "ring-1",
        ring,
        dimensions,
      )}
      aria-hidden="true"
    >
      {initial}
    </div>
  );
}
