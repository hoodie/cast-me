<script lang="ts">
  import { onMount } from "svelte";
  import { initPC } from "./peering";
  import {
    oppositePeerId,
    oppositePeerLeftReason,
    messageHistory
  } from "./stores";

  $: polite = false;
  $: pc = undefined;
  $: transceiver = undefined;

  let videoTag; //: HTMLVideoElement;

  function createPC() {
    pc = initPC(polite);
    pc.ontrack = ({ streams: [stream] }) => {
      videoTag.srcObject = stream;
      videoTag.play();
      console.debug("ontrack", stream);
    };
  }

  async function share() {
    const stream = await navigator.mediaDevices.getDisplayMedia();
    // const stream = await navigator.mediaDevices.getUserMedia({video: true});
    console.debug("share", { stream });
    const [track] = stream.getTracks();
    if (false && transceiver) {
      // console.debug('set sender', {stream, transceiver});
      // transceiver.sender.setStreams(stream) // doesn't work in firefox
    } else {
      transceiver = pc.addTransceiver(track, { streams: [stream] });
      console.debug("new transceiver", { stream, transceiver });
    }
  }

  async function stop() {
    transceiver.stop();
  }

  onMount(() => {
    createPC();
  });
</script>

<section>
  <h4>sharing</h4>
  {#if !pc}
    🔴 peerconnection not available
  {:else}
    <aside>
      {#if $oppositePeerId}
        <label for="polite">
          polite
          <input
            type="checkbox"
            bind:checked={polite}
            name="polite"
            disabled={pc} />
        </label>

        <button on:click={createPC} disabled={!pc}>create pc</button>

        {#if pc}
          <button on:click={share}>▶️ share</button>
          <button on:click={stop}>⏹ stop</button>
        {/if}
      {/if}
    </aside>

    <main>
      <video bind:this={videoTag} autoplay="true">
        <track kind="captions" />
      </video>
    </main>
  {/if}
</section>
