<script>
  import { sendAsRaw } from "./network.js";
  import { oppositePeerId, messageHistory } from './stores';

  $: currentMessage = $oppositePeerId ? `hi "${$oppositePeerId}"` : '';
  const handleSubmit = ({ key }) => key === "Enter" && submit();

  const submit = () => {
    if (!currentMessage.length) return;
    console.debug("sending", currentMessage);
    sendAsRaw(currentMessage);
    currentMessage = "";
  };
</script>

<section>
  {#if $oppositePeerId}
  <label for="my code">
    <h6>chat</h6>
    <ul>
      {#each $messageHistory as message, i (i)}
        <li>{message}</li>
      {/each}
    </ul>
  </label>

  <input
    on:submit={submit}
    on:keydown={handleSubmit}
    bind:value={currentMessage}
    placeholder="chat message"
    type="text" />
  {:else}
  <em> not yet connected </em>
  {/if}
</section>