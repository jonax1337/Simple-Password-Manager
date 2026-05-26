// Plain shadcn-style primitives — no gradients, no glows, no AI-spice.

import * as React from "react";

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
