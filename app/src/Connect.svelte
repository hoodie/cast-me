<script lang="ts">
  import { ownPeerId, sendAsRaw } from "./network";
  import { iInitiatedTheCall, oppositePeerId, oppositePeerLeftReason } from "./stores";

  let connectionCode: string = "";

  const handleSubmit = ({ key }: KeyboardEvent) => key === "Enter" && connect();

  let reloadCountdown: number | undefined;

  const connect = () => {
    if (!!connectionCode) {
      sendAsRaw({ connect: connectionCode.split(" ").join("-") });
      connectionCode = "";
      iInitiatedTheCall.set(true);
    } else {
      console.warn("not connecting");
    }
  };

  const countdownFrom = (seconds: number, then: Function) => {
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
  const unsubConnectionLost = oppositePeerLeftReason.subscribe((reason) => {
    if (reason) {
      console.debug("disconnected, starting reload countdown");
      countdownFrom(5, () => (window.location.href = window.location.href));
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
      <thead>
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
              type="text"
            />
          </td>
        </tr>
        <tr>
          <td> </td>
          <td>
            <strong>↑ enter their code here</strong>
          </td>
        </tr>
      </thead>
    </table>
  </section>
{/if}
