<script>
  import { peerId, history, socket, connected } from "./network.js";

  let connectionCode;

  const handleSubmit = ({ key }) => key === "Enter" && connect();

  const connect = () => {
    if (!!connectionCode) {

      socket.next({ connect: connectionCode.split(' ').join('-') });
      connectionCode = "";
    } else {
      console.warn("not connecting");
    }
  };
</script>

<h2>{$peerId}</h2>
{#if $connected}
  <h5>connected to {$connected}</h5>
{:else}
  not connected
  <label>
    <input
      on:submit={connect}
      on:keydown={handleSubmit}
      bind:value={connectionCode}
      placeholder="connect to"
      type="text" />

  </label>
  {connectionCode}
{/if}
