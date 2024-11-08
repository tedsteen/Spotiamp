<script>
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";

  import { handleError, subscribeToPlayerEvents } from "$lib/common.js";
  import TextTicker from "../../TextTicker.svelte";
  import NumberDisplay from "../../NumberDisplay.svelte";
  import { dispatchWindowChannelEvent } from "$lib/windowChannel";
  // TODO: only export the SpotifyTrack type somehow
  import {
    durationToMMSS,
    durationToString,
    SpotifyTrack,
  } from "$lib/spotifyTrack";
  import { Visualizer } from "$lib/visualizer.svelte";

  /** @type {{data: import('./$types').PageData}} */
  const { data } = $props();
  const { initialVolume } = data;

  /**
   * @type {SpotifyTrack | undefined}
   */
  let loadedTrack = $state();
  let volume = $state(initialVolume);
  let sliderSeekPosition = $state(0);
  let seekPosition = $state(0);
  /**
   * @type {'nothing' | 'seeking' | 'volume-change'}
   */
  let uiInputState = $state("nothing");
  const currentTime = $derived(durationToMMSS(seekPosition));

  subscribeToPlayerEvents(({ payload }) => {
    //console.info("EVENT", payload);
    if (payload.TrackChanged) {
      let { track_uri, artist, name, duration } = payload.TrackChanged;
      const track = new SpotifyTrack(artist, name, duration, track_uri);
      loadTrack(track);
      seekPosition = 0;
    } else if (payload.Playing) {
      let { position_ms } = payload.Playing;
      playerState = "playing";
      seekPosition = position_ms;
    } else if (payload.Paused) {
      let { position_ms } = payload.Paused;
      if (position_ms == 0) {
        playerState = "stopped";
      } else {
        playerState = "paused";
      }
      seekPosition = position_ms;
    } else if (payload.Stopped) {
      playerState = "stopped";
    } else if (payload.PositionCorrection) {
      let { position_ms } = payload.PositionCorrection;
      seekPosition = position_ms;
    } else if (payload.Seeked) {
      let { position_ms } = payload.Seeked;
      seekPosition = position_ms;
    }
  });

  $effect(() => {
    emit("volume-change", volume);
  });

  let tickerOverrideText = $derived.by(() => {
    if (uiInputState == "seeking") {
      return loadedTrack
        ? `SEEK TO: ${durationToString(sliderSeekPosition)}/${loadedTrack.durationAsString} (${Math.ceil((sliderSeekPosition / loadedTrack.durationInMs) * 100)}%)`
        : "NO TRACK LOADED";
    } else if (uiInputState == "volume-change") {
      return `VOLUME: ${volume}%`;
    }
  });

  let numberDisplayHidden = $state(true);

  /**
   * @type {"stopped" | "playing" | "paused"}
   */
  let playerState = $state("stopped");

  /**
   * @param {SpotifyTrack} track
   */
  async function loadTrack(track) {
    loadedTrack = track;
    if (playerState == "playing" || playerState == "paused") {
      await play();
    }
  }

  async function play() {
    if (loadedTrack) {
      if (playerState == "stopped") {
        playerState = "playing"; // To make the UI a bit snappier
        await invoke("load_track", { uri: loadedTrack.uri }).catch(handleError);
      }
    }

    await invoke("play").catch(handleError);
  }

  // handleDrop((url) => {
  //   //TODO: Replace all in playlist with the dropped link
  //   spotifyUrlToTrack(url).then((track) => {
  //     loadAndPlay(track);
  //   });
  // });

  async function pause() {
    playerState = "paused"; // To make the UI a bit snappier
    await invoke("pause").catch(handleError);
  }

  async function stop() {
    playerState = "stopped"; // To make the UI a bit snappier
    await invoke("stop").catch(handleError);
  }

  /**
   * @param {number} positionMs
   */
  async function seek(positionMs) {
    seekPosition = sliderSeekPosition = positionMs; // To make the UI a bit snappier and to not glitch between new and old value
    await invoke("seek", {
      positionMs,
    }).catch(handleError);
  }

  const visualizer = new Visualizer();
  $effect(() => {
    if (playerState == "stopped" || playerState == "paused") {
      visualizer.stop();
    } else if (playerState == "playing") {
      visualizer.start().then(() => {
        if (playerState == "stopped") {
          for (const bar of visualizer.bars) {
            bar.reset();
          }
        }
      });
    }
  });

  // Tick seek position and blink number display
  setInterval(() => {
    if (playerState == "paused") {
      numberDisplayHidden = !numberDisplayHidden;
    } else {
      seekPosition += 1000;
    }
  }, 1000);

  $effect(() => {
    if (uiInputState != "seeking") {
      sliderSeekPosition = seekPosition;
    }
  });

  emit("player-window-ready");
  let showPlaylist = $state(true);
  $effect(() => {
    emit("set-playlist-window-visibility", showPlaylist).catch(handleError);
  });
</script>

<main>
  <div class="sprite main-sprite"></div>

  <div class="sprite stereo-mono-sprite"></div>
  <div
    class="sprite stereo-mono-sprite"
    style="background-position: -29px -12px; --sprite-x: 212px; width: 27px"
  ></div>
  <button
    class="sprite playlist-btn"
    class:playlist-btn-enabled={showPlaylist}
    onclick={() => (showPlaylist = !showPlaylist)}
    aria-label="Toggle playlist"
  ></button>
  <div class="sprite playpause-sprite playpause-{playerState}"></div>

  <div
    data-tauri-drag-region
    class="sprite titlebar-sprite"
    id="titlebar"
  ></div>
  <TextTicker
    text={loadedTrack
      ? `${loadedTrack.artist} - ${loadedTrack.name} (${loadedTrack.durationAsString})`
      : "Winamp 2.91"}
    textOverride={tickerOverrideText}
    x="111"
    y="27"
  />
  <div
    class:hidden={playerState == "stopped" ||
      (playerState == "paused" && numberDisplayHidden)}
  >
    <NumberDisplay
      number={currentTime.m.toString().padStart(2, "0")}
      x="48"
      y="26"
    />
    <NumberDisplay
      number={currentTime.s.toString().padStart(2, "0")}
      x="78"
      y="26"
    />
  </div>
  {#each visualizer.bars as bar}
    <div
      class="visualizer-bar"
      style:--bar-idx={bar.index}
      style:--height={bar.value}
    ></div>
    <div
      class="visualizer-bar-hat"
      style:--bar-idx={bar.index}
      style:--height={bar.hat}
      class:hidden={bar.hat < 0.01}
    ></div>
  {/each}
  <input
    type="range"
    class="sprite volume-sprite"
    style:--volume={volume}
    id="volume"
    min="0"
    max="100"
    bind:value={volume}
    onmousedown={() => (uiInputState = "volume-change")}
    onmouseup={() => (uiInputState = "nothing")}
  />
  <input
    type="range"
    class="sprite seek-position-sprite"
    class:hidden={playerState == "stopped"}
    id="seek-position"
    min="0"
    max={loadedTrack?.durationInMs}
    step="1000"
    bind:value={sliderSeekPosition}
    onmousedown={() => (uiInputState = "seeking")}
    onmouseup={() => {
      seek(sliderSeekPosition);
      uiInputState = "nothing";
    }}
  />

  <button
    class="sprite control-buttons-sprite"
    style:--button-x="calc(16px + (var(--button-width) * 0))"
    style:--button-y="88px"
    style:--button-idx="0"
    onclick={() => dispatchWindowChannelEvent("previous-track")}
    aria-label="Previous"
  ></button>
  <button
    class="sprite control-buttons-sprite"
    style:--button-x="calc(16px + (var(--button-width) * 1))"
    style:--button-y="88px"
    style:--button-idx="1"
    onclick={play}
    aria-label="Play"
  ></button>

  <button
    class="sprite control-buttons-sprite"
    style:--button-x="calc(16px + (var(--button-width) * 2))"
    style:--button-y="88px"
    style:--button-idx="2"
    onclick={pause}
    aria-label="Pause"
  ></button>

  <button
    class="sprite control-buttons-sprite"
    style:--button-x="calc(16px + (var(--button-width) * 3))"
    style:--button-y="88px"
    style:--button-idx="3"
    onclick={stop}
    aria-label="Stop"
  ></button>

  <button
    class="sprite control-buttons-sprite"
    style:--button-x="calc(16px + (var(--button-width) * 4))"
    style:--button-y="88px"
    style:--button-idx="4"
    style:width="22px"
    onclick={() => dispatchWindowChannelEvent("next-track")}
    aria-label="Next"
  ></button>

  <!-- <div
    class="sprite control-buttons-sprite"
    style:--button-width="23px"
    style:--button-x="calc(22px + (var(--button-width) * 5))"
    style:--button-y="89px"
    style:--button-idx="5"
    style:width="21px"
    style:height="16px"
    id="main"
  ></div> -->
</main>

<style>
  button.playlist-btn {
    --sprite-url: url(assets/skins/base-2.91/SHUFREP.BMP);
    --sprite-x: 242px;
    --sprite-y: 58px;
    width: 23px;
    height: 12px;
    background-position: -23px -61px;
  }
  button.playlist-btn:active {
    background-position-x: -69px;
  }

  button.playlist-btn-enabled {
    background-position-y: -73px;
  }

  .stereo-mono-sprite {
    --sprite-url: url(assets/skins/base-2.91/MONOSTER.BMP);
    --sprite-x: 239px;
    --sprite-y: 41px;
    width: 29px;
    height: 12px;
    background-position: 0px 0px;
  }
  /* ------ SEEK POSITION ------ */
  .seek-position-sprite {
    --sprite-url: url(assets/skins/base-2.91/POSBAR.BMP);
    --sprite-x: 16px;
    --sprite-y: 72px;
    width: 249px;
    height: 10px;
    background-position: 0px 0px;
  }

  #seek-position {
    appearance: none;
    cursor: url(assets/skins/base-2.91/VOLBAL.CUR), default;
  }

  #seek-position::-webkit-slider-thumb {
    background: url(assets/skins/base-2.91/POSBAR.BMP);
    appearance: none;
    width: 28px;
    height: 11px;
    margin-bottom: 1px;
    background-position: -249px 11px;
  }

  #seek-position::-webkit-slider-thumb:active {
    background-position: -278px 11px;
  }

  /* ------ /SEEK POSITION ------ */

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
  .visualizer-bar-hat {
    position: absolute;
    --max-height: 16px;
    background: rgb(150, 150, 150);
    top: calc((58px + (1px - var(--height) * var(--max-height))) * var(--zoom));
    width: calc(var(--zoom) * 3px);
    height: calc(var(--zoom) * 1px);
    left: calc((24px + var(--bar-idx) * 4px) * var(--zoom));
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

  .playpause-stopped,
  .playpause-loaded {
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
    background-position-y: calc(
      round(down, var(--volume) / 100 * 27, 1) * -15px
    );
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

  button.control-buttons-sprite {
    border: 0px;
    background-position: calc(var(--button-idx) * var(--button-width) * -1) 0px;
  }

  button.control-buttons-sprite:active {
    background-position: calc(var(--button-idx) * var(--button-width) * -1)
      calc(var(--button-height));
  }

  /* ------ /CBUTTONS ------ */
</style>
