<script lang="ts">
  import Router from "svelte-spa-router";
  import { Toaster, Titlebar } from "$lib/ui";
  import Shell from "./routes/Shell.svelte";
  import EntryEditor from "./routes/EntryEditor.svelte";
  import Settings from "./routes/Settings.svelte";
  import About from "./routes/About.svelte";
  import NotFound from "./routes/NotFound.svelte";

  const routes = {
    "/": Shell,
    "/entry/:uuid": EntryEditor,
    "/settings": Settings,
    "/about": About,
    "*": NotFound,
  } as never;
</script>

<div class="flex flex-col h-full">
  <Titlebar title="Simple Password Manager" />
  <main class="flex-1 min-h-0 overflow-hidden">
    <Router {routes} />
  </main>
</div>

<Toaster />
