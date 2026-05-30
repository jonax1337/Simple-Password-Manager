<script lang="ts" module>
  export type FieldKind = "text" | "password" | "url";
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { Check, Copy, ExternalLink, Eye, EyeOff, Shield } from "@lucide/svelte";
  import { cn } from "$lib/utils";
  import { copyWithFeedback } from "$lib/clipboard";
  import { openUrl } from "$lib/url";
  import IconButton from "./IconButton.svelte";
  import { fieldLabel, bodyText } from "./recipes";

  type Props = {
    label: string;
    value?: string;
    // What to copy when the user clicks the copy action. Defaults to `value`.
    copyValue?: string;
    // Label used in toast + aria text when copying. Defaults to `label`.
    copyLabel?: string;
    kind?: FieldKind;
    // Forces monospace display (passwords, secrets, raw values).
    mono?: boolean;
    // Renders a shield icon next to the label (protected custom fields).
    protected?: boolean;
    // Variant controls the surrounding chrome: `standalone` renders its own
    // border (replaces the old *FieldCard components). `bare` is for rows
    // sitting inside a divided container (the login-details card).
    variant?: "standalone" | "bare";
    class?: string;
    // Optional custom rendering for the value slot — used for TOTP previews
    // or anything beyond a simple string.
    children?: Snippet;
    onCopy?: () => void;
  };

  let {
    label,
    value,
    copyValue,
    copyLabel,
    kind = "text",
    mono = false,
    protected: isProtected = false,
    variant = "standalone",
    class: klass,
    children,
    onCopy,
  }: Props = $props();

  let revealed = $state(false);
  let copied = $state(false);
  let rowEl = $state<HTMLDivElement | null>(null);

  // Auto-mask the password the moment focus leaves the row entirely (Edge /
  // Chromium password field pattern). The check waits for the next
  // microtask so focus can hop between eye-toggle → copy-button without
  // flickering. Hover-out alone is NOT a mask trigger — that would defeat
  // keyboard-only users who tabbed into the reveal.
  function onRowFocusOut(e: FocusEvent) {
    if (!revealed) return;
    const next = e.relatedTarget as Node | null;
    if (!next || !rowEl?.contains(next)) {
      revealed = false;
    }
  }

  const display = $derived(value ?? "");
  const toCopy = $derived(copyValue ?? value ?? "");
  const hasValue = $derived(display.length > 0);
  const hasCopy = $derived(toCopy.length > 0);
  const isPassword = $derived(kind === "password");
  const isUrl = $derived(kind === "url");
  const masked = $derived(
    display ? "•".repeat(Math.min(display.length, 14)) : "",
  );
  const showMono = $derived(mono || isPassword);

  async function doCopy() {
    if (!toCopy) return;
    const ok = await copyWithFeedback(toCopy, copyLabel ?? label);
    if (!ok) return;
    copied = true;
    setTimeout(() => (copied = false), 1500);
    onCopy?.();
  }

  async function doOpenUrl() {
    await openUrl(display);
  }
</script>

<div
  bind:this={rowEl}
  onfocusout={onRowFocusOut}
  class={cn(
    "group/field group/row relative",
    variant === "standalone"
      ? "rounded-lg border bg-card hover:bg-accent/30 transition-colors px-4 py-2.5"
      : "px-4 py-2.5 hover:bg-accent/30 transition-colors",
    klass,
  )}
>
  <div class={cn(fieldLabel, "mb-0.5 flex items-center gap-1.5")}>
    {#if isProtected}<Shield class="size-3" />{/if}
    {label}
  </div>

  <div
    class={cn(
      bodyText,
      "min-h-[18px] break-all",
      showMono && "font-mono",
      // Reserve space for the trailing action buttons so the text doesn't run
      // under them. Two slots for password/url (eye+copy / open+copy), one
      // slot for plain text fields.
      isPassword || isUrl ? "pr-16" : hasCopy ? "pr-9" : "",
    )}
  >
    {#if children}
      {@render children()}
    {:else if !hasValue}
      <span class="text-muted-foreground/50 italic text-xs font-sans">empty</span>
    {:else if isPassword}
      {revealed ? display : masked}
    {:else if isUrl}
      <button type="button" onclick={doOpenUrl} class="text-primary hover:underline text-left">
        {display}
      </button>
    {:else}
      {display}
    {/if}
  </div>

  {#if hasCopy}
    <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-0.5">
      {#if isPassword}
        <IconButton
          size="sm"
          tone="soft"
          softReveal
          onclick={() => (revealed = !revealed)}
          aria-label={revealed ? "Hide password" : "Show password"}
          title={revealed ? "Hide password" : "Show password"}
        >
          {#if revealed}<EyeOff />{:else}<Eye />{/if}
        </IconButton>
      {:else if isUrl}
        <IconButton
          size="sm"
          tone="soft"
          softReveal
          onclick={doOpenUrl}
          aria-label="Open URL"
          title="Open URL"
        >
          <ExternalLink />
        </IconButton>
      {/if}
      <IconButton
        size="sm"
        tone="soft"
        softReveal
        onclick={doCopy}
        aria-label="Copy {copyLabel ?? label}"
        title="Copy {copyLabel ?? label}"
      >
        {#if copied}<Check class="text-success" />{:else}<Copy />{/if}
      </IconButton>
    </div>
  {/if}
</div>
