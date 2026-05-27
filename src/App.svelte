<script lang="ts">
  import { Toaster, Titlebar } from "$lib/ui";
  import Shell from "./routes/Shell.svelte";
</script>

<div class="flex flex-col h-full">
  <Titlebar title="Simple Password Manager" />
  <main class="flex-1 min-h-0 overflow-hidden">
    <Shell />
  </main>
</div>

<Toaster />
