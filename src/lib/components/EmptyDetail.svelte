<script lang="ts">
  import { KeyRound } from "@lucide/svelte";
</script>

<div class="flex h-full flex-col items-center justify-center text-center px-10 select-none bg-background">
  <div class="grid place-items-center size-14 rounded-2xl bg-muted text-muted-foreground mb-5">
    <KeyRound class="size-6" />
  </div>
  <p class="text-[15px] font-semibold tracking-tight">No item selected</p>
  <p class="text-[12.5px] text-muted-foreground mt-1.5 max-w-[260px] leading-relaxed">
    Pick an item from the list, or press
    <kbd class="inline-flex items-center gap-0.5 mx-1 px-1.5 py-0.5 border rounded-md text-[10.5px] bg-card font-mono">
      Ctrl K
    </kbd>
    to search and jump.
  </p>
</div>
