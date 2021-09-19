<script>
  import { onMount } from "svelte";
  import { initPC } from "./peering";

  $: polite = false;
  $: pc = undefined;
  $: transceiver = undefined;

  let videoTag;

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
  <h5>sharing</h5>
  <aside>
    {pc ? '✅' : '🔴'}
    <label for="polite">
      polite
      <input
        type="checkbox"
        bind:checked={polite}
        name="polite"
        disabled={pc} />
    </label>

    <button on:click={createPC} disabled={pc}>create pc</button>

    {#if pc}
      <button on:click={share}>▶️ share</button>
      <button on:click={stop}>⏹ stop</button>
    {/if}
  </aside>

  <main>
    <video bind:this={videoTag} autoplay="true">
      <track kind="captions" />
    </video>
  </main>
</section>
