<script lang="ts">
  import { Check, Copy } from "@lucide/svelte";
  import { previewTotp, type TotpPreview } from "$lib/tauri";
  import { IconButton } from "$lib/ui";
  import { fieldLabel } from "$lib/ui/recipes";
  import { copyWithFeedback } from "$lib/clipboard";

  type Props = { otpUri: string };
  let { otpUri }: Props = $props();

  let preview = $state<TotpPreview | null>(null);
  let copied = $state(false);
  let tick = $state(0);

  $effect(() => {
    if (!otpUri) return;
    const iv = setInterval(() => (tick += 1), 1000);
    return () => clearInterval(iv);
  });

  $effect(() => {
    void tick;
    if (!otpUri) return;
    previewTotp(otpUri).then((p) => (preview = p)).catch(() => (preview = null));
  });

  async function copyCode() {
    if (!preview) return;
    const ok = await copyWithFeedback(preview.code, "TOTP code", { clearAfter: false });
    if (!ok) return;
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  const ratio = $derived(preview && preview.period > 0 ? preview.remaining_seconds / preview.period : 0);
  const urgent = $derived(preview ? preview.remaining_seconds <= 5 : false);
  const dash = $derived(2 * Math.PI * 10 * Math.max(0, Math.min(1, ratio)));
  const rest = $derived(2 * Math.PI * 10 - dash);
</script>

<div class="group/field relative rounded-lg border bg-card hover:bg-accent/30 transition-colors px-4 py-2.5">
  <div class="{fieldLabel} mb-0.5">One-time password</div>
  <div class="min-h-[18px] flex items-center gap-3">
    {#if preview}
      <span class="font-mono text-base tracking-widest tabular-nums">
        {preview.code.slice(0, 3)} {preview.code.slice(3)}
      </span>
      <div class="relative size-5">
        <svg viewBox="0 0 24 24" class="size-5 -rotate-90">
          <circle cx="12" cy="12" r="10" stroke="currentColor" class="text-border" stroke-width="2" fill="none" />
          <circle
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            class={urgent ? "text-destructive" : "text-primary"}
            stroke-width="2"
            stroke-dasharray="{dash} {rest}"
            stroke-linecap="round"
            fill="none"
          />
        </svg>
        <span class="absolute inset-0 grid place-items-center text-[9px] font-medium tabular-nums {urgent ? 'text-destructive' : ''}">
          {preview.remaining_seconds}
        </span>
      </div>
    {:else}
      <span class="text-muted-foreground text-xs">Generating…</span>
    {/if}
  </div>
  {#if preview}
    <IconButton
      size="sm"
      tone="muted"
      softReveal
      onclick={copyCode}
      aria-label="Copy code"
      title="Copy code"
      class="absolute right-2 top-1/2 -translate-y-1/2"
    >
      {#if copied}<Check class="text-success" />{:else}<Copy />{/if}
    </IconButton>
  {/if}
</div>
