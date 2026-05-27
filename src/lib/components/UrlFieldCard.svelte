<script lang="ts">
  import { Check, Copy, ExternalLink } from "@lucide/svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import { toast } from "$lib/ui";

  type Props = { value: string };
  let { value }: Props = $props();

  let copied = $state(false);

  async function doCopy() {
    if (!value) return;
    try {
      await writeText(value);
      copied = true;
      setTimeout(() => (copied = false), 1500);
      toast.success("Copied", "URL copied to clipboard");
    } catch {
      toast.error("Copy failed");
    }
  }

  async function open() {
    if (!value) return;
    try {
      const full = value.match(/^https?:\/\//) ? value : `https://${value}`;
      await openShell(full);
    } catch {
      toast.error("Failed to open URL");
    }
  }
</script>

<div class="group/field relative rounded-lg border bg-card hover:bg-accent/30 transition-colors px-4 py-2.5">
  <div class="text-[10px] font-medium uppercase tracking-wider text-muted-foreground mb-0.5">
    Website
  </div>
  <div class="min-h-[18px] text-sm break-all pr-16">
    {#if value}
      <button type="button" onclick={open} class="text-primary hover:underline text-left">
        {value}
      </button>
    {:else}
      <span class="text-muted-foreground/50 italic text-xs">empty</span>
    {/if}
  </div>
  {#if value}
    <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-0.5">
      <button
        type="button"
        onclick={open}
        class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/field:opacity-100 transition-opacity hover:bg-muted text-muted-foreground hover:text-foreground"
        aria-label="Open URL"
        title="Open URL"
      >
        <ExternalLink class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={doCopy}
        class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/field:opacity-100 transition-opacity hover:bg-muted text-muted-foreground hover:text-foreground"
        aria-label="Copy URL"
        title="Copy URL"
      >
        {#if copied}
          <Check class="size-3.5 text-success" />
        {:else}
          <Copy class="size-3.5" />
        {/if}
      </button>
    </div>
  {/if}
</div>
