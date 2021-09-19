<script lang="ts">
  import { onMount } from "svelte";
  import { initP2P } from "./peering";
  import { iInitiatedTheCall, oppositePeerId } from "./stores";

  $: polite = Boolean($iInitiatedTheCall);
  $: p2p = undefined;
  $: transceiver = undefined;

  let pcInitError;
  let videoTag; //: HTMLVideoElement;

  function createPC() {
    p2p = initP2P({ polite });
    p2p.pc.ontrack = ({ streams: [stream] }) => {
      videoTag.srcObject = stream;
      videoTag.play();
      console.debug("ontrack", stream);
    };
  }

  async function connectP2P() {
    p2p.negotiate();
  }

  async function shareScreen() {
    const stream = await navigator.mediaDevices.getDisplayMedia();
    share(stream);
  }

  async function shareVideo() {
    const stream = await navigator.mediaDevices.getUserMedia({ video: true });
    share(stream);
  }

  async function share(stream: MediaStream) {
    console.debug("share", { stream });
    const [track] = stream.getTracks();
    if (false && transceiver) {
      // console.debug('set sender', {stream, transceiver});
      // transceiver.sender.setStreams(stream) // doesn't work in firefox
    } else {
      transceiver = p2p.pc.addTransceiver(track, { streams: [stream] });
      console.debug("new transceiver", { stream, transceiver });
    }
  }

  async function stop() {
    transceiver.stop();
  }

  onMount(() => {
    try {
      createPC();
    } catch (error) {
      pcInitError = error;
      console.warn("init failed", error);
    }
  });
</script>

{#if $oppositePeerId}
  <section>
    <h4>sharing</h4>
    {#if !p2p}
      🔴 peerconnection not available
    {:else}
      <aside>
        sharing video
        <label for="polite">
          polite
          <input type="checkbox" checked={polite} name="polite" readonly />
        </label>

        <button on:click={connectP2P}>connect p2p</button>

        <button on:click={shareVideo}>▶️ share 📽️</button>
        <button on:click={shareScreen}>▶️ share 🖥️</button>
        <button on:click={stop}>⏹ stop</button>
      </aside>

      <main>
        <video bind:this={videoTag} autoplay={true} controls={true}>
          <track kind="captions" />
        </video>
      </main>
    {/if}
    <main>
      <strong>
        {pcInitError}
      </strong>
    </main>
  </section>
{/if}

<style>
  video {
    width: 100%;
  }
</style>
