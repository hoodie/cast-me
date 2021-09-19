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

<section>
  {#if !$oppositePeerId}
    <em>not yet connected</em>
  {:else if $oppositePeerLeftReason}
    <em>{$oppositePeerLeftReason}</em>
  {:else}
    <label for="my code">
      <h6>chat</h6>
      <ul>
        {#each $messageHistory as message, i (i)}
          <li>
            {#if typeof message === 'string'}{message}{/if}
          </li>
        {/each}
      </ul>
    </label>

    <input
      on:submit={submit}
      on:keydown={handleSubmit}
      bind:value={currentMessage}
      placeholder="chat message"
      type="text" />
  {/if}
</section>
