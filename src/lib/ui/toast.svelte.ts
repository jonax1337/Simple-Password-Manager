export type ToastVariant = "default" | "success" | "error" | "warning";

export type ToastAction = {
  label: string;
  onClick: () => void | Promise<void>;
};

export type Toast = {
  id: number;
  title: string;
  description?: string;
  variant: ToastVariant;
  duration: number;
  action?: ToastAction;
};

let nextId = 1;

// Sonner / 1Password convention: only the top 3 are interactive, anything
// older is auto-dismissed so the stack doesn't grow unbounded after a burst
// of failed copies / saves / etc.
const MAX_VISIBLE = 3;

export const toasts = $state<{ items: Toast[] }>({ items: [] });

function push(t: Omit<Toast, "id">) {
  const id = nextId++;
  toasts.items = [...toasts.items, { id, ...t }];
  // Drop the oldest toast(s) when stack overflows. Errors at the bottom
  // are still discarded — the user can re-trigger from the underlying
  // action, and a stale stack is more harmful than a missed message.
  while (toasts.items.length > MAX_VISIBLE) {
    toasts.items = toasts.items.slice(1);
  }
  if (t.duration > 0) {
    setTimeout(() => dismiss(id), t.duration);
  }
  return id;
}

export function dismiss(id: number) {
  toasts.items = toasts.items.filter((t) => t.id !== id);
}

// Durations per NN/G guidance + research:
//   success / info: brief acknowledgement (3-4s).
//   warning: hold a bit longer (5s) — users need to read it.
//   error: persistent — let the user dismiss explicitly. Otherwise
//          a failure that auto-disappears is a failure that gets ignored.
export const toast = {
  show: (title: string, opts: Partial<Omit<Toast, "id" | "title">> = {}) =>
    push({ title, variant: "default", duration: 3500, ...opts }),
  success: (title: string, description?: string) =>
    push({ title, description, variant: "success", duration: 3500 }),
  error: (title: string, description?: string) =>
    push({ title, description, variant: "error", duration: 0 }),
  warning: (title: string, description?: string) =>
    push({ title, description, variant: "warning", duration: 5000 }),
  action: (
    title: string,
    action: ToastAction,
    opts: Partial<Omit<Toast, "id" | "title" | "action">> = {},
  ) =>
    push({
      title,
      variant: "default",
      duration: 0,
      ...opts,
      action,
    }),
};
