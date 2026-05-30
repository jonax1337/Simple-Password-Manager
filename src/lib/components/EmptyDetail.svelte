<script lang="ts">
  import { KeyRound } from "@lucide/svelte";
  import { EmptyState, Kbd } from "$lib/ui";
</script>

<div class="h-full bg-background">
  <EmptyState
    icon={KeyRound}
    title="No item selected"
    description="Pick an item from the list, or press the shortcut to search."
  >
    <div class="flex items-center justify-center gap-1.5 text-2xs text-muted-foreground">
      <span>Open palette</span>
      <Kbd>Ctrl</Kbd>
      <Kbd>K</Kbd>
    </div>
  </EmptyState>
</div>
