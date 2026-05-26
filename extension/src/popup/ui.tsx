// Plain shadcn-style primitives — no gradients, no glows, no AI-spice.

import * as React from "react";
import { Check } from "lucide-react";

export function cn(...classes: (string | false | null | undefined)[]) {
  return classes.filter(Boolean).join(" ");
}

// ---------- Button ----------

type ButtonVariant = "default" | "outline" | "ghost" | "secondary" | "destructive";
type ButtonSize = "default" | "sm" | "icon";

const buttonBase =
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";

const buttonVariants: Record<ButtonVariant, string> = {
  default: "bg-primary text-primary-foreground shadow hover:bg-primary/90",
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
  sm: "h-8 px-3 text-xs",
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
        "rounded-lg border bg-card text-card-foreground shadow-sm",
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
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background",
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

// ---------- Checkbox ----------
// shadcn-style: 16px square that flips bg-primary + check icon when on.
// Hidden native input keeps it form-submittable and keyboard-friendly.

export function Checkbox(props: {
  checked: boolean;
  onCheckedChange: (v: boolean) => void;
  id?: string;
  className?: string;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={props.checked}
      id={props.id}
      onClick={() => props.onCheckedChange(!props.checked)}
      className={cn(
        "flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-primary",
        "transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        props.checked
          ? "bg-primary text-primary-foreground"
          : "bg-background hover:bg-accent",
        props.className,
      )}
    >
      {props.checked && <Check className="h-3 w-3" strokeWidth={3} />}
    </button>
  );
}

// ---------- Slider ----------
// Range input with the native thumb hidden, an overlay track + filled
// portion + circular thumb rendered in shadcn tokens. Drag still works
// because the native input sits on top with opacity-0.

export function Slider(props: {
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
  className?: string;
}) {
  const ratio = (props.value - props.min) / (props.max - props.min);
  return (
    <div className={cn("relative flex h-5 w-full items-center", props.className)}>
      {/* track */}
      <div className="absolute inset-x-0 h-1.5 rounded-full bg-muted" />
      {/* filled portion */}
      <div
        className="pointer-events-none absolute left-0 h-1.5 rounded-full bg-primary"
        style={{ width: `${ratio * 100}%` }}
      />
      {/* thumb */}
      <div
        className="pointer-events-none absolute h-4 w-4 -translate-x-1/2 rounded-full border-2 border-primary bg-background shadow-sm"
        style={{ left: `${ratio * 100}%` }}
      />
      {/* invisible native range eats the drag input */}
      <input
        type="range"
        min={props.min}
        max={props.max}
        value={props.value}
        onChange={(e) => props.onChange(Number(e.target.value))}
        className="relative z-10 h-5 w-full cursor-pointer appearance-none bg-transparent opacity-0"
      />
    </div>
  );
}

// ---------- Letter Badge ----------
// Solid muted square with the entry's first letter. No gradients.

export function LetterBadge(props: { title: string; size?: "sm" | "md" }) {
  const initial = (props.title.trim()[0] ?? "?").toUpperCase();
  const dimensions =
    props.size === "md" ? "h-9 w-9 text-sm" : "h-7 w-7 text-xs";
  return (
    <div
      className={cn(
        "flex shrink-0 items-center justify-center rounded-md bg-muted font-semibold text-muted-foreground",
        dimensions,
      )}
      aria-hidden="true"
    >
      {initial}
    </div>
  );
}
