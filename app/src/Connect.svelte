<script lang="ts">
  import { ownPeerId, sendAsRaw } from "./network";
  import { oppositePeerId, oppositePeerLeftReason } from "./stores";

  let connectionCode;

  const handleSubmit = ({ key }) => key === "Enter" && connect();

  let reloadCountdown;

  const connect = () => {
    if (!!connectionCode) {
      sendAsRaw({ connect: connectionCode.split(" ").join("-") });
      connectionCode = "";
    } else {
      console.warn("not connecting");
    }
  };

  const countdownFrom = (seconds: number, then: function) => {
    reloadCountdown = seconds;
    if (seconds > 0) {
      console.debug("countdown", seconds);
      setTimeout(() => {
        countdownFrom(seconds - 1, then);
      }, 1000);
    } else {
      then();
    }
  };
  // reload when disconnected
  const unsubConnectionLost = oppositePeerLeftReason.subscribe(reason => {
    if (reason) {
      console.debug("disconnected, starting reload countdown");
      countdownFrom(5, () => window.location = window.location);
      unsubConnectionLost();
    }
  });
</script>

<section>
  {#if $oppositePeerId}
    {#if $oppositePeerLeftReason}
      <h6>🔴 {$oppositePeerLeftReason}</h6>
    {:else}
      <h6>✅ connected</h6>
    {/if}
  {:else}
    <h6>⏳ not yet connected</h6>
  {/if}
  {#if reloadCountdown}reloading in {reloadCountdown} seconds{/if}
</section>

{#if !$oppositePeerId}
  <section>
    <h4>connect to other peer</h4>

    <table>
      <tr>
        <td>
          <input type="text" value={$ownPeerId} readonly />
        </td>
        <td>

          <input
            on:submit={connect}
            on:keydown={handleSubmit}
            bind:value={connectionCode}
            placeholder="enter opposite peerId"
            type="text" />

        </td>
      </tr>
      <tr>
        <td />
        <td>
          <strong>↑ enter their code here</strong>
        </td>
      </tr>
    </table>

  </section>
{/if}
