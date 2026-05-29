<script lang="ts">
  import { Popover, Button } from "$lib/ui";
  import { KEEPASS_ICONS, getIconById } from "$lib/keepass-icons";

  type Props = {
    value: number;
    onChange: (id: number) => void;
  };

  let { value, onChange }: Props = $props();
  let open = $state(false);

  const selected = $derived(getIconById(value));
</script>

<Popover bind:open class="w-80 p-2">
  {#snippet trigger()}
    <Button variant="outline" size="icon" type="button" title={selected.description}>
      <selected.icon class="h-4 w-4" />
    </Button>
  {/snippet}

  <div class="mb-2 text-xs text-muted-foreground px-1">Standard Icons (69)</div>
  <div class="grid grid-cols-8 gap-1 h-64 overflow-y-auto pr-2">
    {#each KEEPASS_ICONS as ic (ic.id)}
      <Button
        variant={value === ic.id ? "default" : "ghost"}
        size="icon"
        type="button"
        onclick={() => {
          onChange(ic.id);
          open = false;
        }}
        title={`${ic.id}: ${ic.description}`}
      >
        <ic.icon class="h-4 w-4" />
      </Button>
    {/each}
  </div>
</Popover>
