<script lang="ts">
  import { Button, Input, Label } from "$lib/ui";
  import { Check, Copy, KeyRound, Trash2 } from "@lucide/svelte";
  import { previewTotp, type TotpPreview, type EntryData, type CustomField } from "$lib/tauri";
  import { copyWithFeedback } from "$lib/clipboard";

  type Props = {
    formData: EntryData;
    onUpdate: (next: EntryData) => void;
  };

  let { formData, onUpdate }: Props = $props();

  const OTP_FIELD = "otp";
  const otpField = $derived(
    (formData.custom_fields ?? []).find((f) => f.name.toLowerCase() === OTP_FIELD) ?? null,
  );

  // svelte-ignore state_referenced_locally
  let editing = $state(!otpField);
  // svelte-ignore state_referenced_locally
  let input = $state(otpField?.value ?? "");
  let preview = $state<TotpPreview | null>(null);
  let error = $state<string | null>(null);
  let validating = $state(false);
  let copied = $state(false);
  let tick = $state(0);
  let savedCode = $state<TotpPreview | null>(null);

  $effect(() => {
    editing = !otpField;
    input = otpField?.value ?? "";
    error = null;
    preview = null;
  });

  $effect(() => {
    if (!editing) return;
    if (!input.trim()) {
      preview = null;
      error = null;
      return;
    }
    const t = setTimeout(async () => {
      validating = true;
      try {
        const r = await previewTotp(input);
        preview = r;
        error = null;
      } catch (e) {
        preview = null;
        error = String(e);
      } finally {
        validating = false;
      }
    }, 200);
    return () => clearTimeout(t);
  });

  $effect(() => {
    const showing = !editing && otpField;
    if (!showing) return;
    const t = setInterval(() => (tick += 1), 1000);
    return () => clearInterval(t);
  });

  $effect(() => {
    if (editing || !otpField) return;
    // re-evaluate every tick
    void tick;
    previewTotp(otpField.value).then((r) => (savedCode = r)).catch(() => (savedCode = null));
  });

  function commit(uri: string) {
    const others = (formData.custom_fields ?? []).filter((f) => f.name.toLowerCase() !== OTP_FIELD);
    const next: CustomField = { name: OTP_FIELD, value: uri, protected: true };
    onUpdate({ ...formData, custom_fields: [...others, next] });
    editing = false;
  }

  function remove() {
    onUpdate({
      ...formData,
      custom_fields: (formData.custom_fields ?? []).filter((f) => f.name.toLowerCase() !== OTP_FIELD),
    });
    editing = true;
    input = "";
    preview = null;
    error = null;
  }

  async function copyCode() {
    const code = savedCode?.code ?? preview?.code;
    if (!code) return;
    const ok = await copyWithFeedback(code, "TOTP code", { clearAfter: false });
    if (!ok) return;
    copied = true;
    setTimeout(() => (copied = false), 1200);
  }
</script>

<div class="rounded-md border bg-card p-3 space-y-2">
  <div class="flex items-center gap-2">
    <KeyRound class="h-4 w-4 text-muted-foreground" />
    <Label class="text-sm font-medium">Two-factor authentication (TOTP)</Label>
  </div>

  {#if !editing && otpField && savedCode}
    {@const ratio = savedCode.period > 0 ? savedCode.remaining_seconds / savedCode.period : 0}
    {@const urgent = savedCode.remaining_seconds <= 5}
    <div class="flex items-center gap-3">
      <div class="flex flex-1 items-center justify-between rounded-md border bg-background px-3 py-2">
        <span class="font-mono text-lg tracking-widest tabular-nums">
          {savedCode.code.slice(0, 3)} {savedCode.code.slice(3)}
        </span>
        <div class="relative h-6 w-6">
          <svg viewBox="0 0 24 24" class="h-6 w-6 -rotate-90">
            <circle cx="12" cy="12" r="10" stroke="currentColor" class="text-border" stroke-width="2" fill="none" />
            <circle
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              class={urgent ? "text-destructive" : "text-primary"}
              stroke-width="2"
              stroke-dasharray="{2 * Math.PI * 10 * Math.max(0, Math.min(1, ratio))} {2 * Math.PI * 10 - 2 * Math.PI * 10 * Math.max(0, Math.min(1, ratio))}"
              stroke-linecap="round"
              fill="none"
            />
          </svg>
          <span class="absolute inset-0 flex items-center justify-center text-2xs font-medium tabular-nums {urgent ? 'text-destructive' : ''}">
            {savedCode.remaining_seconds}
          </span>
        </div>
      </div>
      <Button size="sm" variant="outline" onclick={copyCode}>
        {#if copied}<Check class="h-3.5 w-3.5" />{:else}<Copy class="h-3.5 w-3.5" />{/if}
      </Button>
      <Button size="sm" variant="outline" onclick={() => (editing = true)}>Reconfigure</Button>
      <Button size="sm" variant="ghost" onclick={remove} title="Remove 2FA">
        <Trash2 class="h-3.5 w-3.5 text-destructive" />
      </Button>
    </div>
  {:else if !editing && otpField && !savedCode}
    <p class="text-xs text-muted-foreground">Reading current code…</p>
  {/if}

  {#if editing}
    <div class="space-y-2">
      <p class="text-xs text-muted-foreground leading-relaxed">
        Paste an <code class="font-mono">otpauth://</code> URI from the site's 2FA setup screen, or the secret string.
      </p>
      <div class="flex items-stretch gap-1">
        <Input
          bind:value={input}
          placeholder="otpauth://totp/... or JBSWY3DPEHPK3PXP"
          class="font-mono text-xs"
          spellcheck={false}
        />
        <Button size="sm" onclick={() => preview && commit(preview.otpauth_uri)} disabled={!preview || validating}>
          Save 2FA
        </Button>
        {#if otpField}
          <Button size="sm" variant="outline" onclick={() => { editing = false; input = otpField.value; error = null; }}>
            Cancel
          </Button>
        {/if}
      </div>

      {#if preview}
        <div class="flex items-center justify-between rounded border border-success/30 bg-success/5 px-2.5 py-2">
          <div class="flex items-center gap-2 text-xs">
            <span class="text-success font-medium">Looks good:</span>
            <span class="font-mono text-base tracking-widest">{preview.code.slice(0, 3)} {preview.code.slice(3)}</span>
            <span class="text-muted-foreground">· refreshes in {preview.remaining_seconds}s</span>
          </div>
        </div>
      {/if}

      {#if error}
        <p class="text-xs text-destructive">Not valid: {error}</p>
      {/if}
      {#if validating && !preview && !error}
        <p class="text-xs text-muted-foreground">Validating…</p>
      {/if}
    </div>
  {/if}
</div>
