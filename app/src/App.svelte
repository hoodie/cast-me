<script>
  import Chat from "./Chat.svelte";
  import Connect from "./Connect.svelte";
  import { connected } from "./socketStore.js";
</script>

<style>
  main {
    text-align: center;
    padding: 1em;
    max-width: 240px;
    margin: 0 auto;
  }

  @media (min-width: 640px) {
    main {
      max-width: none;
    }
  }
</style>

<main>
  <Connect />
  <Chat />
</main>
