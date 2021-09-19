<script lang="ts">
  import Chat from "./Chat.svelte";
  import Connect from "./Connect.svelte";
  import Share from "./Share.svelte";
</script>

<main>
  <Connect />
  <Chat />
  <Share />
</main>

<style>
  main {
    text-align: center;
    padding: 1em;
    max-width: 240px;
    margin: 0 auto;
  }

  @media (min-width: 640px) {
    main {
      max-width: 880px;
    }
  }
</style>
