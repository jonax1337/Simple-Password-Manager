<script lang="ts">
  import { KeyRound } from "@lucide/svelte";
</script>

<div class="flex h-full flex-col items-center justify-center text-center px-8 select-none">
  <div class="grid place-items-center size-14 rounded-2xl bg-muted/60 mb-4">
    <KeyRound class="size-6 text-muted-foreground" />
  </div>
  <p class="text-sm font-medium">No item selected</p>
  <p class="text-xs text-muted-foreground mt-1 max-w-xs">
    Pick an entry from the list, or press <kbd class="border rounded px-1 py-0.5 text-[10px] bg-muted/60">Ctrl K</kbd> to search.
  </p>
</div>
