<script lang="ts">
  import { Check, Copy, Eye, EyeOff } from "@lucide/svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { toast } from "$lib/ui";

  type Props = { value: string };
  let { value }: Props = $props();

  let revealed = $state(false);
  let copied = $state(false);

  const masked = $derived("•".repeat(Math.min(value.length, 14) || 6));

  async function doCopy() {
    if (!value) return;
    try {
      await writeText(value);
      copied = true;
      setTimeout(() => (copied = false), 1500);
      toast.success("Copied", "Password copied to clipboard");
      setTimeout(() => void writeText(""), 30000);
    } catch {
      toast.error("Copy failed");
    }
  }
</script>

<div class="group/field relative rounded-lg border bg-card hover:bg-accent/30 transition-colors px-4 py-2.5">
  <div class="text-[10px] font-medium uppercase tracking-wider text-muted-foreground mb-0.5">
    Password
  </div>
  <div class="min-h-[18px] text-sm font-mono break-all pr-16">
    {#if value}
      {revealed ? value : masked}
    {:else}
      <span class="text-muted-foreground/50 italic text-xs">empty</span>
    {/if}
  </div>
  {#if value}
    <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-0.5">
      <button
        type="button"
        onclick={() => (revealed = !revealed)}
        class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/field:opacity-100 transition-opacity hover:bg-muted text-muted-foreground hover:text-foreground"
        aria-label={revealed ? "Hide password" : "Show password"}
        title={revealed ? "Hide password" : "Show password"}
      >
        {#if revealed}
          <EyeOff class="size-3.5" />
        {:else}
          <Eye class="size-3.5" />
        {/if}
      </button>
      <button
        type="button"
        onclick={doCopy}
        class="size-7 inline-flex items-center justify-center rounded-md opacity-0 group-hover/field:opacity-100 transition-opacity hover:bg-muted text-muted-foreground hover:text-foreground"
        aria-label="Copy password"
        title="Copy password"
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
