<script lang="ts">
  import { sendAsRaw } from "./network";
  import {
    oppositePeerId,
    oppositePeerLeftReason,
    messageHistory
  } from "./stores";

  $: currentMessage = $oppositePeerId ? `hi "${$oppositePeerId}"` : "";
  const handleSubmit = ({ key }: KeyboardEvent) => key === "Enter" && submit();

  const submit = () => {
    if (!currentMessage.length) return;
    console.debug("sending", currentMessage);
    sendAsRaw(currentMessage);
    currentMessage = "";
  };
</script>

{#if !$oppositePeerId}

{:else if $oppositePeerLeftReason}
  <section>
    <em>{$oppositePeerLeftReason}</em>
  </section>
{:else}
  <section>
    <label for="my code">
      <h5>chat</h5>
      <ul>
        {#each $messageHistory as message, i (i)}
          {#if typeof message === 'string'}
            <li>{message}</li>
          {/if}
        {/each}
      </ul>
    </label>

    <input
      on:submit={submit}
      on:keydown={handleSubmit}
      bind:value={currentMessage}
      placeholder="chat message"
      type="text" />
  </section>
{/if}
