<script lang="ts">
  import { KeyRound } from "@lucide/svelte";
</script>

<div class="flex h-full flex-col items-center justify-center text-center px-10 select-none bg-background">
  <div class="relative">
    <div class="absolute -inset-8 rounded-full bg-primary/[0.06] blur-2xl" aria-hidden="true"></div>
    <div class="relative grid place-items-center size-16 rounded-2xl bg-primary/10 text-primary mb-5 shadow-xs">
      <KeyRound class="size-7" />
    </div>
  </div>
  <p class="text-[15px] font-semibold tracking-tight">No item selected</p>
  <p class="text-[12.5px] text-muted-foreground mt-1.5 max-w-[260px] leading-relaxed">
    Pick an item from the list, or press
    <kbd class="inline-flex items-center gap-0.5 mx-1 px-1.5 py-0.5 border rounded-md text-[10.5px] bg-card/80 font-mono shadow-xs">
      Ctrl K
    </kbd>
    to search and jump.
  </p>
</div>
