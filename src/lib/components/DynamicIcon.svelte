<script lang="ts">
  import { getIconComponent } from "$lib/keepass-icons";

  type Props = { iconId: number; class?: string };
  let { iconId, class: klass }: Props = $props();

  const Icon = $derived(getIconComponent(iconId));
</script>

<Icon class={klass} />
