<script lang="ts">
  import type { Snippet } from "svelte";
  import { Check, Copy } from "@lucide/svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { toast } from "$lib/ui";

  type Props = {
    label: string;
    value?: string;
    copyValue?: string;
    copyLabel?: string;
    mono?: boolean;
    children?: Snippet;
    onCopy?: () => void;
  };

  let { label, value, copyValue, copyLabel, mono = false, children, onCopy }: Props = $props();

  let copied = $state(false);
  const displayValue = $derived(value ?? "");
  const toCopy = $derived(copyValue ?? value ?? "");
  const hasCopy = $derived(toCopy.length > 0);

  async function doCopy() {
    if (!toCopy) return;
    try {
      await writeText(toCopy);
      copied = true;
      setTimeout(() => (copied = false), 1500);
      toast.success("Copied", `${copyLabel ?? label} copied to clipboard`);
      setTimeout(() => void writeText(""), 30000);
      onCopy?.();
    } catch {
      toast.error("Copy failed");
    }
  }
</script>

<div class="group/field relative rounded-lg border bg-card hover:bg-accent/30 transition-colors px-4 py-2.5">
  <div class="text-[10px] font-medium uppercase tracking-wider text-muted-foreground mb-0.5">
    {label}
  </div>
  <div class="min-h-[18px] text-sm break-all {mono ? 'font-mono' : ''}">
    {#if children}
      {@render children()}
    {:else if displayValue}
      {displayValue}
    {:else}
      <span class="text-muted-foreground/50 italic text-xs">empty</span>
    {/if}
  </div>
  {#if hasCopy}
    <button
      type="button"
      onclick={doCopy}
      class="absolute right-2 top-1/2 -translate-y-1/2 size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/field:opacity-100 transition-opacity hover:bg-muted text-muted-foreground hover:text-foreground"
      aria-label="Copy {copyLabel ?? label}"
      title="Copy {copyLabel ?? label}"
    >
      {#if copied}
        <Check class="size-3.5 text-success" />
      {:else}
        <Copy class="size-3.5" />
      {/if}
    </button>
  {/if}
</div>
