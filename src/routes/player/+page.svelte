<script>
  import { invoke } from "@tauri-apps/api/core";

  import {
    handleError,
    handleDrop,
    REACTIVE_WINDOW_SIZE,
  } from "$lib/common.svelte.js";
  import {
    emitWindowEvent,
    subscribeToWindowEvent,
  } from "$lib/events.svelte.js";
  import {
    durationToMMSS,
    durationToString,
    SpotifyTrack,
  } from "$lib/spotify.svelte.js";
  import TextTicker from "../../TextTicker.svelte";
  import NumberDisplay from "../../NumberDisplay.svelte";
  import { onMount } from "svelte";
  import { Visualizer } from "$lib/visualizer.svelte";
  import {
    currentMonitor,
    getCurrentWindow,
    Window,
  } from "@tauri-apps/api/window";
  import {
    boundingRect,
    isDocked,
    makeTauriWindowDraggable,
    rectFromPositionAndSize,
    SNAP_DISTANCE,
    snapPosition,
    snapRectIntoBounds,
    STICKY_SNAP_DISTANCE,
  } from "$lib/window-docking.svelte.js";

  /** @type {{data: import('./$types').PageData}} */
  const { data: playerSettings } = $props();

  function initialVolume() {
    return playerSettings.volume;
  }

  function initialShowPlaylist() {
    return playerSettings.show_playlist;
  }

  function initialDoubleSizeActive() {
    return playerSettings.double_size_active;
  }

  /**
   * @type {SpotifyTrack | undefined}
   */
  let loadedTrack = $state();
  let volume = $state(initialVolume());
  let sliderSeekPosition = $state(0);
  let seekPosition = $state(0);
  // Wall-clock anchor for interpolating the playback position between backend
  // updates. Advancing seekPosition by elapsed real time (rather than a fixed
  // +1s per tick) keeps the clock from drifting when timers fire irregularly.
  let positionAnchorMs = 0;
  let positionAnchorAt = 0;

  /**
   * Set the playback position and re-anchor the interpolation clock to now.
   * @param {number} positionMs
   */
  function setPosition(positionMs) {
    seekPosition = positionMs;
    positionAnchorMs = positionMs;
    positionAnchorAt = performance.now();
  }
  let showPlaylist = $state(initialShowPlaylist());
  let doubleSizeActive = $state(initialDoubleSizeActive());
  /**
   * @type {'nothing' | 'seeking' | 'volume-change'}
   */
  let uiInputState = $state("nothing");
  /**
   * @type {"unavailable" | "stopped" | "playing" | "paused"}
   */
  let playerState = $state("stopped");
  let numberDisplayHidden = $state(true);

  const currentTime = $derived(durationToMMSS(seekPosition));
  const tickerOverrideText = $derived.by(() => {
    if (uiInputState == "seeking") {
      return loadedTrack
        ? `SEEK TO: ${durationToString(sliderSeekPosition)}/${loadedTrack.displayDuration} (${Math.ceil((sliderSeekPosition / loadedTrack.durationInMs) * 100)}%)`
        : "NO TRACK LOADED";
    } else if (uiInputState == "volume-change") {
      return `VOLUME: ${volume}%`;
    }
  });

  /**
   * @param {SpotifyTrack} track
   */
  async function loadTrack(track) {
    loadedTrack = track;
    if (playerState != "stopped") {
      playerState = "stopped";
      await play();
    }
  }

  async function play() {
    if (playerState == "paused") {
      await invoke("play").catch(handleError);
    } else if (loadedTrack) {
      setPosition(0);
      sliderSeekPosition = 0;
      playerState = loadedTrack.unavailable ? "unavailable" : "playing";

      if (playerState == "unavailable") {
        await invoke("stop").catch(handleError);
      } else {
        await invoke("load_track", { uri: loadedTrack?.uri.asString }).catch(
          handleError,
        );
      }
    }
  }

  async function pause() {
    if (playerState == "playing") {
      playerState = "paused"; // To make the UI a bit snappier
      await invoke("pause").catch(handleError);
    }
  }

  async function stop() {
    setPosition(0);
    sliderSeekPosition = 0;
    playerState = "stopped"; // To make the UI a bit snappier
    await invoke("stop").catch(handleError);
  }

  /**
   * @param {number} positionMs
   */
  async function seek(positionMs) {
    // To make the UI a bit snappier and to not glitch between new and old value
    setPosition(positionMs);
    sliderSeekPosition = positionMs;
    await invoke("seek", {
      positionMs,
    }).catch(handleError);
  }

  const visualizer = new Visualizer();
  $effect(() => {
    if (playerState != "playing") {
      visualizer.stop(playerState == "stopped" || playerState == "unavailable");
    } else {
      visualizer.start();
    }
  });

  $effect(() => {
    invoke("set_volume", { volume });
  });

  $effect(() => {
    if (uiInputState != "seeking") {
      sliderSeekPosition = seekPosition;
    }
  });

  $effect(() => {
    invoke("set_playlist_window_visible", {
      visible: showPlaylist,
    }).catch(handleError);
  });

  $effect(() => {
    invoke("set_double_size", { active: doubleSizeActive });
    REACTIVE_WINDOW_SIZE.setZoom(doubleSizeActive ? 2 : 1);
  });

  onMount(() => {
    // Tick seek position and blink number display
    const tickerInterval = setInterval(() => {
      if (playerState == "paused") {
        numberDisplayHidden = !numberDisplayHidden;
      } else if (playerState != "unavailable") {
        seekPosition =
          positionAnchorMs + (performance.now() - positionAnchorAt);
      }
    }, 1000);

    const playlistWindowEventSubscription = subscribeToWindowEvent(
      "playlistWindow",
      (event) => {
        if (event.TrackLoaded) {
          let track = event.TrackLoaded;
          loadTrack(track);
        } else if (event.PlayRequested !== undefined) {
          play();
        } else if (event.EndReached !== undefined) {
          stop();
        }
      },
    );

    const playerEventsSubscription = subscribeToWindowEvent(
      "player",
      (event) => {
        if (event.Playing) {
          const { position_ms } = event.Playing;
          playerState = "playing";
          setPosition(position_ms);
        } else if (event.Paused) {
          const { position_ms } = event.Paused;
          playerState = "paused";
          setPosition(position_ms);
        } else if (event.Stopped) {
          playerState = "stopped";
        } else if (event.PositionCorrection) {
          const { position_ms } = event.PositionCorrection;
          setPosition(position_ms);
        } else if (event.PositionChanged) {
          const { position_ms } = event.PositionChanged;
          setPosition(position_ms);
        } else if (event.Seeked) {
          const { position_ms } = event.Seeked;
          setPosition(position_ms);
        }
      },
    );

    const cleanupDropHandler = handleDrop((urls) => {
      emitWindowEvent("playerWindow", { UrlsDropped: urls });
    });

    return () => {
      clearInterval(tickerInterval);
      playerEventsSubscription.then((unlisten) => unlisten());
      playlistWindowEventSubscription.then((unlisten) => unlisten());
      cleanupDropHandler();
    };
  });

  /**
   * @param {HTMLElement} element
   */
  function makeWindowDraggable(element) {
    makeTauriWindowDraggable(element, {
      async onStart({ startPosition, windowSize }) {
        const playlistWindow = await Window.getByLabel("playlist");
        const [playlistPosition, playlistSize, monitor] = await Promise.all([
          playlistWindow?.outerPosition(),
          playlistWindow?.outerSize(),
          currentMonitor(),
        ]);
        const playlistRect =
          playlistPosition && playlistSize
            ? rectFromPositionAndSize(playlistPosition, playlistSize)
            : undefined;
        const startRect = rectFromPositionAndSize(startPosition, windowSize);
        const dockedAtStart = playlistRect
          ? isDocked(startRect, playlistRect)
          : false;

        return {
          docked: dockedAtStart,
          dockedAtStart,
          groupStartRect:
            dockedAtStart && playlistRect
              ? boundingRect([startRect, playlistRect])
              : startRect,
          playlistRect,
          screenBounds: monitor
            ? rectFromPositionAndSize(
                monitor.workArea.position,
                monitor.workArea.size,
              )
            : undefined,
          screenSnapped: false,
        };
      },
      mapPosition(rawPosition, context, { startPosition, windowSize }) {
        let position = rawPosition;
        if (context.playlistRect && !context.dockedAtStart) {
          const rawRect = {
            ...rawPosition,
            width: windowSize.width,
            height: windowSize.height,
          };
          const snapDistance = context.docked
            ? STICKY_SNAP_DISTANCE
            : SNAP_DISTANCE;
          const snappedPosition = snapPosition(
            rawRect,
            context.playlistRect,
            snapDistance,
          );
          position = snappedPosition ?? rawPosition;
          context.docked = snappedPosition !== undefined;
        }

        if (context.screenBounds) {
          const movingGroupRect = {
            x: context.groupStartRect.x + position.x - startPosition.x,
            y: context.groupStartRect.y + position.y - startPosition.y,
            width: context.groupStartRect.width,
            height: context.groupStartRect.height,
          };
          const snappedGroupPosition = snapRectIntoBounds(
            movingGroupRect,
            context.screenBounds,
            context.screenSnapped ? STICKY_SNAP_DISTANCE : SNAP_DISTANCE,
          );

          if (snappedGroupPosition) {
            position = {
              x: position.x + snappedGroupPosition.x - movingGroupRect.x,
              y: position.y + snappedGroupPosition.y - movingGroupRect.y,
            };
          }
          context.screenSnapped = snappedGroupPosition !== undefined;
        }

        return position;
      },
      async onEnd() {
        await emitWindowEvent("playerWindow", { DragEnded: null });
      },
    });
  }
</script>

<main>
  <div class="sprite main-sprite"></div>

  <div class="sprite stereo-mono-sprite stereo-mono-sprite-mono"></div>
  <div
    class="sprite stereo-mono-sprite stereo-mono-sprite-stereo"
    class:stereo-mono-sprite-enabled={playerState != "stopped" &&
      playerState != "unavailable"}
  ></div>

  <button
    class="sprite playlist-btn"
    class:playlist-btn-enabled={showPlaylist}
    onclick={() => (showPlaylist = !showPlaylist)}
    aria-label="Toggle playlist"
  ></button>
  <div class="sprite playpause-sprite playpause-{playerState}"></div>

  <div
    use:makeWindowDraggable
    class="sprite titlebar-sprite"
    id="titlebar"
  ></div>

  <button
    class="sprite close-btn"
    onclick={() => emitWindowEvent("playerWindow", { CloseRequested: null })}
    aria-label="Close"
  ></button>
  <button
    class="sprite minimize-btn"
    onclick={() => getCurrentWindow().minimize()}
    aria-label="Minimize"
  ></button>

  <div class="sprite side-buttons"></div>
  <button
    class="sprite double-size-btn"
    onclick={() => (doubleSizeActive = !doubleSizeActive)}
    class:active={doubleSizeActive}
    aria-label="Toggle double size"
  ></button>

  <TextTicker
    unavailable={playerState == "unavailable"}
    text={loadedTrack
      ? `${loadedTrack.displayName} (${loadedTrack.displayDuration})`
      : "Winamp 2.91"}
    textOverride={tickerOverrideText}
    x="111"
    y="27"
  />
  <div
    class:hidden={playerState == "stopped" ||
      playerState == "unavailable" ||
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
    style:--volume-row={Math.floor((volume / 100) * 27)}
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
    class:hidden={playerState == "stopped" || playerState == "unavailable"}
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
    onclick={() => emitWindowEvent("playerWindow", { PreviousPressed: null })}
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
    onclick={() => emitWindowEvent("playerWindow", { NextPressed: null })}
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
  button.close-btn {
    cursor: url(/src/static/assets/skins/base-2.91/CLOSE.CUR), default;
    --sprite-url: url(/src/static/assets/skins/base-2.91/TITLEBAR.BMP);
    --sprite-x: 264px;
    --sprite-y: 3px;
    width: 9px;
    height: 9px;
    background-position: -18px 0px;
  }

  button.close-btn:active {
    background-position-y: -9px;
  }

  button.minimize-btn {
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), auto;
    --sprite-url: url(/src/static/assets/skins/base-2.91/TITLEBAR.BMP);
    --sprite-x: 244px;
    --sprite-y: 3px;
    width: 9px;
    height: 9px;
    background-position: -9px 0px;
  }

  button.minimize-btn:active {
    background-position-y: -9px;
  }

  .side-buttons {
    --sprite-url: url(/src/static/assets/skins/base-2.91/TITLEBAR.BMP);
    --sprite-x: 10px;
    --sprite-y: 22px;
    width: 8px;
    height: 43px;
    background-position: -304px 0px;
  }

  button.double-size-btn {
    --sprite-url: url(/src/static/assets/skins/base-2.91/TITLEBAR.BMP);
    --sprite-x: 10px;
    --sprite-y: 48px;
    width: 8px;
    height: 8px;
    background-position: -328px -70px;
    opacity: 0;
  }
  button.double-size-btn.active {
    opacity: 1;
  }

  button.playlist-btn {
    --sprite-url: url(/src/static/assets/skins/base-2.91/SHUFREP.BMP);
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
    --sprite-url: url(/src/static/assets/skins/base-2.91/MONOSTER.BMP);
    --sprite-y: 41px;
    height: 12px;
  }

  .stereo-mono-sprite-mono {
    --sprite-x: 212px;
    width: 27px;
    background-position: -29px -12px;
  }

  .stereo-mono-sprite-stereo {
    --sprite-x: 239px;
    width: 29px;
    background-position: 0px -12px;
  }

  .stereo-mono-sprite-enabled {
    background-position-y: 0px;
  }

  /* ------ SEEK POSITION ------ */
  .seek-position-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/POSBAR.BMP);
    --sprite-x: 16px;
    --sprite-y: 72px;
    width: 249px;
    height: 10px;
    background-position: 0px 0px;
  }

  #seek-position {
    appearance: none;
    cursor: url(/src/static/assets/skins/base-2.91/VOLBAL.CUR), default;
  }

  #seek-position::-webkit-slider-thumb {
    background: url(/src/static/assets/skins/base-2.91/POSBAR.BMP);
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
    left: calc((24px + var(--bar-idx) * 4px) * var(--zoom));
    width: calc(var(--zoom) * 3px);

    --max-height: 16px;
    top: calc((59px - var(--max-height) * var(--height)) * var(--zoom));
    height: calc(var(--max-height) * var(--height) * var(--zoom));

    background: linear-gradient(
      rgb(213, 76, 0) 0% 6.67%,
      rgb(213, 89, 0) 6.67% 13.34%,
      rgb(215, 102, 0) 13.34% 20.009999999999998%,
      rgb(214, 115, 1) 20.009999999999998% 26.68%,
      rgb(197, 124, 4) 26.68% 33.35%,
      rgb(222, 165, 21) 33.35% 40.019999999999996%,
      rgb(213, 181, 34) 40.019999999999996% 46.69%,
      rgb(189, 222, 42) 46.69% 53.36%,
      rgb(148, 221, 34) 53.36% 60.03%,
      rgb(41, 206, 16) 60.03% 66.7%,
      rgb(50, 190, 16) 66.7% 73.37%,
      rgb(56, 181, 17) 73.37% 80.03999999999999%,
      rgb(49, 156, 6) 80.03999999999999% 86.71%,
      rgb(40, 148, 1) 86.71% 93.38%,
      rgb(27, 132, 6) 93.38% 100.05%
    );
    background-position: bottom;
    background-repeat: no-repeat;
    background-size: 100% calc(var(--max-height) * var(--zoom));
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
    --sprite-url: url(/src/static/assets/skins/base-2.91/TITLEBAR.BMP);
    width: 275px;
    height: 14px;
    background-position: -27px 0px;
    cursor: url(/src/static/assets/skins/base-2.91/TITLEBAR.CUR), default;
  }

  /* ------ /TITLEBAR ------ */

  /* ------ MAIN ------ */
  .main-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/MAIN.BMP);
    width: 275px;
    height: 116px;
    background-position: 0px 0px;
  }

  /* ------ /MAIN ------ */

  /* ------ PLAYPAUSE ------ */
  .playpause-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLAYPAUS.BMP);
    width: 9px;
    height: 9px;
    --sprite-x: 26px;
    --sprite-y: 28px;
  }
  .playpause-playing,
  .playpause-unavailable {
    background-position: -0px 0px;
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
    --sprite-url: url(/src/static/assets/skins/base-2.91/VOLUME.BMP);
    --sprite-x: 107px;
    --sprite-y: 57px;
    width: 65px;
    height: 14px;
    background-position: 0px 0px;
  }

  #volume {
    appearance: none;
    cursor: url(/src/static/assets/skins/base-2.91/VOLBAL.CUR), default;
    background-position-y: calc(var(--volume-row) * -15px);
  }

  #volume::-webkit-slider-thumb {
    background: url(/src/static/assets/skins/base-2.91/VOLUME.BMP);
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
    --sprite-url: url(/src/static/assets/skins/base-2.91/CBUTTONS.BMP);
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
