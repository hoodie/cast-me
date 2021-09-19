<script lang="ts">
  import { ownPeerId, sendAsRaw } from "./network";
  import { oppositePeerId, oppositePeerLeftReason } from "./stores";

  let connectionCode;

  const handleSubmit = ({ key }) => key === "Enter" && connect();

  const connect = () => {
    if (!!connectionCode) {
      sendAsRaw({ connect: connectionCode.split(" ").join("-") });
      connectionCode = "";
    } else {
      console.warn("not connecting");
    }
  };
</script>

<style>
  h3 > code {
    font-size: 2.6em;
  }
</style>

<section>
  {#if $oppositePeerId}
    {#if $oppositePeerLeftReason}
      <h6>🔴 {$oppositePeerLeftReason}</h6>
    {:else}
      <h6>✅ connected</h6>
    {/if}
  {:else}
    <h3>
      peerId:
      <code>{$ownPeerId}</code>
    </h3>
    <code>⏳ not connected</code>
    <label>
      <input
        on:submit={connect}
        on:keydown={handleSubmit}
        bind:value={connectionCode}
        placeholder="enter opposite peerId"
        type="text" />

    </label>
  {/if}
</section>
