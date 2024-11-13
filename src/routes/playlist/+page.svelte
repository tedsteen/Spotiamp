<script>
  import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
  import {
    enterExitViewport,
    setZoom,
    range,
    handleDrop,
    emitWindowEvent,
    SpotifyUri,
  } from "$lib/common.js";
  import { onMount } from "svelte";
  import { Playlist } from "$lib/playlist.svelte";

  const ZOOM = 1;
  setZoom(ZOOM);

  let playlistWidth = $state(Math.ceil(window.innerWidth / ZOOM / 25));
  let playlistHeight = $state(Math.ceil(window.innerHeight / ZOOM / 29));
  const playlist = new Playlist();

  /**
   * @argument {string[]} var_args
   */
  async function loadUrls(...var_args) {
    for (var url of var_args) {
      await playlist.addRow(SpotifyUri.fromUrl(url));
    }
  }
  onMount(() => {
    loadUrls(
      "https://open.spotify.com/track/4rZSduTjZIZIcAY2bW7H0l",
      "https://open.spotify.com/track/5ezjAnO0uuGL10qvOe1tCT",
      "https://open.spotify.com/track/6ZZHFLjVpsilHYyv3mLuVe",
      "https://open.spotify.com/playlist/2XWjC6cK8YAy3QtrwH9h7a",
      /* "https://open.spotify.com/playlist/2zKOYCC7MRak6klBtBCO5G" */
    );

    const cleanupDropHandler = handleDrop((url) => {
      playlist.addRow(SpotifyUri.fromUrl(url));
    });

    emitWindowEvent("playlistWindow", { Ready: null });

    // Cleanups
    return () => {
      cleanupDropHandler();
      playlist.dispose();
    };
  });

  /**
   * @param {HTMLElement} element
   */
  function makeDraggable(element) {
    element.onpointerdown = function (event) {
      document.onmousemove = function (event) {
        const pointerX = Math.max(Math.ceil(event.clientX / ZOOM / 25), 11);
        const pointerY = Math.max(Math.ceil(event.clientY / ZOOM / 29), 4);
        const w = getCurrentWindow();
        playlistWidth = pointerX;
        playlistHeight = pointerY;

        w.setSize(new LogicalSize(pointerX * 25 * ZOOM, pointerY * 29 * ZOOM));
      };

      document.onmouseup = function () {
        document.onmousemove = null;

        element.releasePointerCapture(event.pointerId);
      };

      element.setPointerCapture(event.pointerId);
    };

    element.onselectstart = () => false;
  }
</script>

<span style:--playlist-w={playlistWidth} style:--playlist-h={playlistHeight}>
  <div class="tracks-container">
    <table id="playlist-tracks">
      <tbody>
        {#each playlist.rows as row, index}
          <tr
            class="playlist-track"
            class:loaded={row.isLoaded()}
            class:selected={row.isSelected()}
            class:unavailable={row.unavailable}
            onmousedown={() => (playlist.selectedRows = [row])}
            ondblclick={() => row.play()}
            use:enterExitViewport
            onenterViewport={row.getOnEnterViewport()}
          >
            <td>
              <span class="playlist-track-number">{index + 1}.&nbsp;</span>
              <span class="playlist-track-name">{row.displayName}</span>
            </td>
            <td class="playlist-track-duration">{row.displayDuration}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <!-- Top corners -->
  <div class="sprite playlist-sprite playlist-tl-sprite"></div>

  <div
    class="sprite playlist-sprite playlist-tr-sprite"
    style:--x={playlistWidth}
  ></div>

  <!-- Left/Right -->
  {#each range(1, playlistHeight - 2) as y}
    <div class="sprite playlist-sprite playlist-l-sprite" style:--y={y}></div>
    <div
      class="sprite playlist-sprite playlist-r-sprite"
      style:--y={y}
      style:--x={playlistWidth}
    ></div>
  {/each}

  <!-- Top/Bottom -->
  {#each range(1, playlistWidth - 2) as x}
    <div
      data-tauri-drag-region
      class="sprite playlist-sprite playlist-t-sprite"
      style:--x={x}
    ></div>
    {#if x >= 5 && x < playlistWidth - 6}
      <div
        class="sprite playlist-sprite playlist-b-sprite"
        style:--y={playlistHeight - 1}
        style:--x={x}
      ></div>
    {/if}
  {/each}

  <!-- Title -->
  <div
    data-tauri-drag-region
    class="sprite playlist-sprite playlist-title-sprite"
    style:--x={playlistWidth / 2 - 2}
  ></div>

  <!-- Bottom corners -->
  <div
    class="sprite playlist-sprite playlist-bl-sprite"
    style:--y={playlistHeight}
  ></div>

  <div
    class="sprite playlist-sprite playlist-br-sprite"
    style:--y={playlistHeight - 1}
    style:--x={playlistWidth - 9}
  ></div>

  <div class="draggable-corner" use:makeDraggable></div>
</span>

<style>
  @font-face {
    font-family: px sans nouveaux;
    font-style: normal;
    font-weight: 400;
    src:
      local("px sans nouveaux"),
      url(/src/static/assets/px_sans_nouveaux.woff) format("woff");
  }

  .draggable-corner {
    cursor: url(/src/static/assets/skins/base-2.91/TITLEBAR.CUR), default;
    --width: 15px;
    --height: 15px;
    width: calc(var(--width) * var(--zoom));
    height: calc(var(--height) * var(--zoom));
    background-color: transparent;
    position: absolute;
    --x: var(--playlist-w);
    --y: var(--playlist-h);
    left: calc(((var(--x)) * 25px - var(--width)) * var(--zoom));
    top: calc(((var(--y)) * 29px - var(--height)) * var(--zoom));
    display: inline-block;
  }
  /* ------ TRACKS ------ */
  .tracks-container {
    --track-row-height: 14.5px;
    margin-top: calc(20px * var(--zoom));
    margin-left: calc(10px * var(--zoom));
    width: calc((var(--playlist-w) * 25px - 29px) * var(--zoom));
    height: calc(
      (var(--playlist-h) - 1) * 2 * var(--track-row-height) * var(--zoom)
    );
    overflow: hidden;
  }

  #playlist-tracks {
    color: rgb(0, 255, 0);
    border-collapse: collapse;
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(7px * var(--zoom));
    font-smooth: never;
    -webkit-font-smoothing: none;

    letter-spacing: calc(0.3px * var(--zoom));
    -webkit-user-select: none;
    -ms-user-select: none;
    user-select: none;
    width: 100%;
  }

  .playlist-track {
    outline: none;
    height: calc(var(--track-row-height) * var(--zoom));
  }

  .playlist-track-number {
    padding-left: calc(3px * var(--zoom));
  }

  .playlist-track-duration {
    padding-right: calc(5px * var(--zoom));
    text-align: right;
  }

  .playlist-track.selected {
    background-color: rgb(0, 0, 198);
  }

  .playlist-track.loaded {
    color: white;
  }

  .playlist-track.unavailable {
    color: rgb(80, 80, 80);
  }

  .playlist-track.unavailable.loaded {
    color: rgb(140, 140, 140);
  }

  /* ------ /TRACKS ------ */

  /* ------ PLAYLIST ------ */
  .playlist-sprite {
    --x: 0;
    --y: 0;
    --sprite-x: calc(var(--x) * 25px);
    --sprite-y: calc(var(--y) * 20px);
  }

  .playlist-tl-sprite {
    cursor: url(/src/static/assets/skins/base-2.91/TITLEBAR.CUR), default;
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 20px;
  }

  .playlist-t-sprite {
    cursor: url(/src/static/assets/skins/base-2.91/TITLEBAR.CUR), default;
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 20px;
    --y: 0;
    --sprite-x: calc(var(--x) * 25px);
    background-position: -127px 0px;
  }

  .playlist-title-sprite {
    cursor: url(/src/static/assets/skins/base-2.91/TITLEBAR.CUR), default;
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 100px;
    height: 20px;
    --y: 0;
    --sprite-x: calc(var(--x) * 25px);
    background-position: -26px 0px;
  }

  .playlist-tr-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 20px;
    --x: var(--playlist-w);
    --y: 0;
    --sprite-x: calc((var(--x) - 1) * 25px);
    background-position: -153px 0px;
  }

  .playlist-l-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 10px;
    height: 29px;
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: 0px -42px;
  }

  .playlist-r-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 19px;
    height: 29px;
    --x: var(--playlist-w);
    --sprite-x: calc((var(--x) - 1) * 25px + 6px);
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: -32px -42px;
  }

  .playlist-bl-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 125px;
    height: 38px;
    --y: var(--playlist-h);
    --sprite-y: calc((var(--y) - 1) * 29px - 9px);
    background-position: 0px -72px;
  }

  .playlist-b-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 38px;
    --y: var(--playlist-h);
    --sprite-x: calc(var(--x) * 25px);
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: -179px 0px;
  }

  .playlist-br-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    width: 150px;
    height: 38px;
    --x: var(--playlist-w);
    --y: var(--playlist-h);
    --sprite-x: calc(var(--x) * 25px + 75px);
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: 154px -72px;
  }
  /* ------ /PLAYLIST ------ */
</style>
