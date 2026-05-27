<script lang="ts">
  import { Button } from "$lib/ui";
  import { push } from "svelte-spa-router";
</script>

<div class="h-full grid place-items-center">
  <div class="text-center space-y-3">
    <h1 class="text-4xl font-bold">404</h1>
    <p class="text-muted-foreground">Page not found</p>
    <Button onclick={() => push("/")}>Go home</Button>
  </div>
</div>
