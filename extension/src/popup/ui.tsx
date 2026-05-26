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
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:pointer-events-none disabled:opacity-50";

const buttonVariants: Record<ButtonVariant, string> = {
  default: "bg-primary text-primary-foreground shadow-sm hover:bg-primary/90",
  outline:
    "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
  ghost: "hover:bg-accent hover:text-accent-foreground",
  secondary:
    "bg-secondary text-secondary-foreground hover:bg-secondary/80",
  destructive:
    "bg-destructive text-destructive-foreground hover:bg-destructive/90",
};

const buttonSizes: Record<ButtonSize, string> = {
  default: "h-9 px-3 py-2",
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
        "rounded-md border border-border bg-card text-card-foreground shadow-sm",
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
      "flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm transition-colors",
      "placeholder:text-muted-foreground",
      "focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
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

const PALETTE = [
  ["bg-indigo-500/20", "text-indigo-600 dark:text-indigo-300"],
  ["bg-rose-500/20", "text-rose-600 dark:text-rose-300"],
  ["bg-emerald-500/20", "text-emerald-600 dark:text-emerald-300"],
  ["bg-amber-500/20", "text-amber-600 dark:text-amber-300"],
  ["bg-sky-500/20", "text-sky-600 dark:text-sky-300"],
  ["bg-violet-500/20", "text-violet-600 dark:text-violet-300"],
  ["bg-pink-500/20", "text-pink-600 dark:text-pink-300"],
  ["bg-teal-500/20", "text-teal-600 dark:text-teal-300"],
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
  const [bg, fg] = PALETTE[hashTitle(props.title) % PALETTE.length];
  const dimensions =
    props.size === "md" ? "h-9 w-9 text-sm" : "h-7 w-7 text-xs";
  return (
    <div
      className={cn(
        "flex shrink-0 items-center justify-center rounded-md font-semibold",
        dimensions,
        bg,
        fg,
      )}
      aria-hidden="true"
    >
      {initial}
    </div>
  );
}
