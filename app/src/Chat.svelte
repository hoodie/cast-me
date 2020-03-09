<script>
  import { peerId, history, socket } from "./network.js";

  let currentMessage = "hi";
  const handleSubmit = ({ key }) => key === "Enter" && submit();

  const submit = () => {
    if (!currentMessage.length) return;
    console.debug("sending", currentMessage);
    socket.next(currentMessage);
    currentMessage = "";
  };
</script>

<label for="my code">
  messages
  <ul>
    {#each $history as message, i (i)}
      <li>{message}</li>
    {/each}
  </ul>
</label>

<input
  on:submit={submit}
  on:keydown={handleSubmit}
  bind:value={currentMessage}
  type="text" />
