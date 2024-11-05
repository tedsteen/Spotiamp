<script>
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";

  import { handleError, handleDrop, spotifyUrlToTrack } from "$lib/common.js";
  import TextTicker from "../../TextTicker.svelte";
  import NumberDisplay from "../../NumberDisplay.svelte";
  import {
    dispatchWindowChannelEvent,
    subscribeToWindowChannelEvent,
  } from "$lib/windowChannel";
  // TODO: only export the type somehow
  import { SpotifyTrack } from "$lib/spotifyTrack";

  /** @type {{data: import('./$types').PageData}} */
  const { data } = $props();
  const { initialVolume } = data;

  let volume = $state(initialVolume);

  $effect(() => {
    emit("volume-change", volume);
  });

  let volumeYOffs = $derived(-Math.floor((volume / 100.0) * 27) * 15);
  let tickerText = $state("Winamp 2.91");
  let tickerOverrideEnabled = $state(false);
  let tickerOverrideText = $derived(
    tickerOverrideEnabled ? `VOLUME: ${volume}%` : undefined,
  );
  let numberDisplayHidden = $state(true);

  /**
   * @type {"stopped" | "playing" | "paused"}
   */
  let playerState = $state("stopped");

  /**
   * @type {SpotifyTrack | undefined}
   */
  let loadedTrack;

  /**
   * @param {SpotifyTrack} track
   */
  function loadTrack(track) {
    tickerText = `${track.artist} - ${track.name} (${track.durationAsString})`;
    loadedTrack = track;
    playerState = "stopped";
  }

  async function play() {
    let trackToStartPlaying = loadedTrack;
    if (playerState == "paused") {
      trackToStartPlaying = undefined; //Don't start playing the loadedTrack, just resume the play
    }

    await invoke("play", { uri: trackToStartPlaying?.uri }).catch(handleError);
    playerState = "playing";
  }

  /**
   * @param {SpotifyTrack} track
   */
  async function loadAndPlay(track) {
    loadTrack(track);
    play().catch(handleError);
  }

  handleDrop((url) => {
    //TODO: Replace all in playlist with this
    spotifyUrlToTrack(url).then((track) => {
      loadAndPlay(track);
    });
  });

  async function pause() {
    if (playerState == "playing") {
      await invoke("pause").catch(handleError);
      playerState = "paused";
    }
  }

  async function stop() {
    if (playerState != "stopped") {
      await invoke("stop").catch(handleError);
      playerState = "stopped";
    }
  }

  subscribeToWindowChannelEvent("load-track", (track) => {
    console.info("load-track", track);
    if (playerState != "stopped") {
      loadAndPlay(track);
    } else {
      loadTrack(track);
    }
  });

  subscribeToWindowChannelEvent("play-track", (track) => {
    console.info("play-track", track);
    loadAndPlay(track);
  });

  let minutes = $state(0);
  let seconds = $state(0);

  // Send a player ready at startup
  dispatchWindowChannelEvent("player-ready");
  // And then respond on every ping...
  subscribeToWindowChannelEvent("ping-player", () => {
    dispatchWindowChannelEvent("player-ready");
  });

  const INITIAL_VISUALIZER_STATE = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  ];
  let visualizerState = $state(INITIAL_VISUALIZER_STATE);

  let visualizerRunning = false;
  async function startVisualizer() {
    visualizerRunning = true;
    while (visualizerRunning) {
      try {
        const visualizerData = await invoke("take_latest_spectrum", {});
        if (visualizerData) {
          let idx = 0;
          for (var i of visualizerData) {
            visualizerState[idx] = Math.min(i[1], 1);
            idx++;
          }
        }
      } catch (e) {
        console.error("Failed to fetch visualizer data", e);
      }
    }
    visualizerState = INITIAL_VISUALIZER_STATE;
  }

  /**
   * @type number | undefined
   */
  let playOrPauseInterval;
  $effect(() => {
    clearInterval(playOrPauseInterval);
    if (playerState == "paused" || playerState == "playing") {
      playOrPauseInterval = setInterval(() => {
        if (playerState == "paused") {
          numberDisplayHidden = !numberDisplayHidden;
        } else {
          seconds++;
          if (seconds == 60) {
            seconds = 0;
            minutes++;
          }
        }
      }, 1000);
    }

    if (playerState == "stopped") {
      numberDisplayHidden = true;
      seconds = minutes = 0;
      visualizerRunning = false;
    } else if (playerState == "playing") {
      numberDisplayHidden = false;
      startVisualizer();
    }
  });
</script>

<svelte:head>
  <title>Player</title>
</svelte:head>

<main class="container">
  <div class="sprite main-sprite"></div>
  <div class="sprite playpause-sprite playpause-{playerState}"></div>

  <div
    data-tauri-drag-region
    class="sprite titlebar-sprite"
    id="titlebar"
  ></div>
  <TextTicker
    text={tickerText}
    textOverride={tickerOverrideText}
    x="111"
    y="27"
  />
  <div class:hidden={numberDisplayHidden}>
    <NumberDisplay number={minutes.toString().padStart(2, "0")} x="48" y="26" />
    <NumberDisplay number={seconds.toString().padStart(2, "0")} x="78" y="26" />
  </div>
  {#each visualizerState as s, i}
    <div class="visualizer-bar" style="--bar-idx: {i}; --height: {s};"></div>
  {/each}
  <input
    type="range"
    class="sprite volume-sprite"
    style="background-position-y: {volumeYOffs}px"
    id="volume"
    min="0"
    max="100"
    bind:value={volume}
    onmousedown={() => (tickerOverrideEnabled = true)}
    onmouseup={() => (tickerOverrideEnabled = false)}
  />
  <input
    type="button"
    class="sprite control-buttons-sprite"
    style="--button-x: calc(16px + (var(--button-width) * 0)); --button-y: 88px; --button-idx: 0;"
    onclick={() => dispatchWindowChannelEvent("previous-track")}
  />
  <input
    type="button"
    class="sprite control-buttons-sprite"
    style="--button-x: calc(16px + (var(--button-width) * 1)); --button-y: 88px; --button-idx: 1;"
    onclick={play}
  />

  <input
    type="button"
    class="sprite control-buttons-sprite"
    style="--button-x: calc(16px + (var(--button-width) * 2)); --button-y: 88px; --button-idx: 2;"
    onclick={pause}
  />

  <input
    type="button"
    class="sprite control-buttons-sprite"
    style="--button-x: calc(16px + (var(--button-width) * 3)); --button-y: 88px; --button-idx: 3;"
    onclick={stop}
  />

  <input
    type="button"
    class="sprite control-buttons-sprite"
    style="--button-x: calc(16px + (var(--button-width) * 4)); --button-y: 88px; --button-idx: 4; width: 22px; "
    onclick={() => dispatchWindowChannelEvent("next-track")}
  />

  <!-- <div class="sprite control-buttons-sprite"
    style="--button-width: 23px; --button-x: calc(22px + (var(--button-width) * 5)); --button-y: 89px; --button-idx: 5; width: 21px; height: 16px; "
    id="main">
  </div> -->
</main>

<style>
  /* ------ VISUALIZER ------ */
  .visualizer-bar {
    position: absolute;
    display: inline-block;
    left: calc((24px + var(--bar-idx) * 4px) * var(--zoom));
    width: calc(var(--zoom) * 3px);

    --max-height: 16px;
    top: calc((59px - var(--max-height)) * var(--zoom));
    height: calc(var(--max-height) * var(--zoom));

    clip-path: rect(
      calc(var(--zoom) * var(--max-height) * (1 - var(--height))) auto auto auto
    );

    background: linear-gradient(
      rgb(213 76 0) 0% 6.67%,
      rgb(213 89 0) 6.67% 13.34%,
      rgb(215 102 0) 13.34% 20.009999999999998%,
      rgb(214 115 1) 20.009999999999998% 26.68%,
      rgb(197 124 4) 26.68% 33.35%,
      rgb(222 165 21) 33.35% 40.019999999999996%,
      rgb(213 181 34) 40.019999999999996% 46.69%,
      rgb(189 222 42) 46.69% 53.36%,
      rgb(148 221 34) 53.36% 60.03%,
      rgb(41 206 16) 60.03% 66.7%,
      rgb(50 190 16) 66.7% 73.37%,
      rgb(56 181 17) 73.37% 80.03999999999999%,
      rgb(49 156 6) 80.03999999999999% 86.71%,
      rgb(40 148 1) 86.71% 93.38%,
      rgb(27 132 6) 93.38% 100.05%
    );
  }
  /* ------ /VISUALIZER ------ */

  /* ------ TITLEBAR ------ */
  .titlebar-sprite {
    --sprite-url: url(assets/skins/base-2.91/TITLEBAR.BMP);
    width: 275px;
    height: 14px;
    background-position: -27px 0px;
    cursor: url(assets/skins/base-2.91/TITLEBAR.CUR), default;
  }

  /* ------ /TITLEBAR ------ */

  /* ------ MAIN ------ */
  .main-sprite {
    --sprite-url: url(assets/skins/base-2.91/MAIN.BMP);
    width: 275px;
    height: 116px;
    background-position: 0px 0px;
  }

  /* ------ /MAIN ------ */

  /* ------ PLAYPAUSE ------ */
  .playpause-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLAYPAUS.BMP);
    width: 9px;
    height: 9px;
    --sprite-x: 26px;
    --sprite-y: 28px;
  }

  .playpause-paused {
    background-position: -9px 0px;
  }

  .playpause-stopped {
    background-position: -18px 0px;
  }

  /* ------ /PLAYPAUSE ------ */

  /* ------ VOLUME ------ */
  .volume-sprite {
    --sprite-url: url(assets/skins/base-2.91/VOLUME.BMP);
    --sprite-x: 107px;
    --sprite-y: 57px;
    width: 65px;
    height: 14px;
    background-position: 0px 0px;
  }

  #volume {
    appearance: none;
    cursor: url(assets/skins/base-2.91/VOLBAL.CUR), default;
  }

  #volume::-webkit-slider-thumb {
    background: url(assets/skins/base-2.91/VOLUME.BMP);
    appearance: none;
    width: 14px;
    height: 11px;
    margin-bottom: 1px;
    background-position: -15px 11px;
  }

  #volume::-webkit-slider-thumb:active {
    background-position: 0px 11px;
  }

  /* ------ /VOLUME ------ */

  /* ------ CBUTTONS ------ */
  .control-buttons-sprite {
    --sprite-url: url(assets/skins/base-2.91/CBUTTONS.BMP);
    --button-width: 23px;
    --button-height: 18px;
    --button-state: 0;
    width: var(--button-width);
    height: var(--button-height);
    background-position: 0px 0px;
    left: calc(var(--button-x) * var(--zoom));
    top: calc(var(--button-y) * var(--zoom));
  }

  input[type="button"],
  .control-buttons-sprite {
    border: 0px;
    background-position: calc(var(--button-idx) * var(--button-width) * -1) 0px;
  }

  input[type="button"]:active,
  .control-buttons-sprite {
    background-position: calc(var(--button-idx) * var(--button-width) * -1)
      calc(var(--button-height));
  }

  /* ------ /CBUTTONS ------ */
</style>
