<script lang="ts">
  import { calculatePasswordStrength } from "$lib/password-strength";
  import { cn } from "$lib/utils";

  type Props = { password: string; class?: string };
  let { password, class: klass }: Props = $props();

  const strength = $derived(calculatePasswordStrength(password));
  const textColor = $derived(strength.bits < 80 ? "#000000" : "#ffffff");
  const widthPct = $derived(Math.min((strength.bits / 128) * 100, 100));
</script>

<div class={cn("relative", klass)}>
  <div class="h-6 w-full rounded-md bg-secondary/20 overflow-hidden border border-border">
    {#if password}
      <div
        class="h-full transition-all duration-300 flex items-center justify-end px-2"
        style="width: {widthPct}%; background: {strength.gradient}; min-width: 50px;"
      >
        <span class="text-xs font-semibold whitespace-nowrap" style="color: {textColor};">
          {strength.bits} bits
        </span>
      </div>
    {:else}
      <div class="h-full flex items-center justify-center">
        <span class="text-xs text-muted-foreground">No password</span>
      </div>
    {/if}
  </div>
</div>
