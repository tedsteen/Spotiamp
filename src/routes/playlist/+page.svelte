<script>
  import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
  import {
    dispatchPlaylistEvent,
    SpotifyTrack,
    spotifyUrlToTrack,
    setZoom,
    range,
    subscribeToPlaylistEvent,
    handleDrop,
  } from "$lib/common.js";

  const ZOOM = 1;
  setZoom(ZOOM);
  let loadedRow = $state();

  class PlaylistRow {
    /**
     * @param {SpotifyTrack} track
     */
    constructor(track) {
      this.track = track;
    }

    load() {
      loadedRow = this;
      dispatchPlaylistEvent("load-track", this.track);
    }

    play() {
      dispatchPlaylistEvent("play-track", this.track);
      loadedRow = this;
    }

    isLoaded() {
      return this == loadedRow;
    }

    isSelected() {
      return selectedRows.indexOf(this) != -1;
    }
  }

  /**
   * @type {PlaylistRow[]}
   */
  let rows = $state([]);

  /**
   * @type {PlaylistRow[]}
   */
  let selectedRows = $state([]);

  document.addEventListener("keydown", (e) => {
    const selectedRow = selectedRows[0];
    if (selectedRow) {
      let nextRow;
      if (e.key == "ArrowDown") {
        const currRowIndex = rows.indexOf(selectedRow);
        nextRow = rows[currRowIndex + 1];
      } else if (e.key == "ArrowUp") {
        const currRowIndex = rows.indexOf(selectedRow);
        nextRow = rows[currRowIndex - 1];
      }
      if (nextRow) {
        selectedRows = [nextRow];
      }
    }
  });

  /**
   * @param {SpotifyTrack} track
   */
  function addTrack(track) {
    const row = new PlaylistRow(track);
    rows.push(row);
    if (!loadedRow) {
      row.load();
    }
  }

  handleDrop((url) => {
    spotifyUrlToTrack(url).then((track) => {
      addTrack(track);
    });
  });

  subscribeToPlaylistEvent("next-track", () => {
    const currRowIndex = rows.indexOf(loadedRow);
    const nextRow = rows[currRowIndex + 1];
    console.info("next-track", currRowIndex, nextRow);
    nextRow?.load();
  });

  subscribeToPlaylistEvent("previous-track", () => {
    const currRowIndex = rows.indexOf(loadedRow);
    const previousRow = rows[currRowIndex - 1];
    console.info("previous-track", currRowIndex, previousRow);
    previousRow?.load();
  });

  spotifyUrlToTrack(
    "https://open.spotify.com/track/6zTO0Y58ZBd1ZMjH0EIX1X",
  ).then((track) => {
    addTrack(track);
  });
  spotifyUrlToTrack(
    "https://open.spotify.com/track/72oaFIAqlK7N7a8cyHZZ3i",
  ).then((track) => {
    addTrack(track);
  });
  spotifyUrlToTrack(
    "https://open.spotify.com/track/6qnoOnDK3embwtU89Fz5XN",
  ).then((track) => {
    addTrack(track);
  });

  class Size {
    width = $state(Math.ceil(window.innerWidth / ZOOM / 25));
    height = $state(Math.ceil(window.innerHeight / ZOOM / 29));
  }
  const size = new Size();

  /**
   * @param {HTMLElement} element
   */
  function makeDraggable(element) {
    element.onpointerdown = function (event) {
      document.onmousemove = function (event) {
        const pointerX = Math.max(Math.ceil(event.clientX / ZOOM / 25), 11);
        const pointerY = Math.max(Math.ceil(event.clientY / ZOOM / 29), 4);
        const w = getCurrentWindow();
        size.width = pointerX;
        size.height = pointerY;

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

<svelte:head>
  <title>Playlist</title>
</svelte:head>

<span style:--playlist-w={size.width} style:--playlist-h={size.height}>
  <div class="tracks-container">
    <table id="playlist-tracks">
      <tbody>
        {#each rows as row, index}
          <tr
            class="playlist-track"
            class:loaded={row.isLoaded()}
            class:selected={row.isSelected()}
            onmousedown={() => (selectedRows = [row])}
            ondblclick={() => row.play()}
          >
            <td>
              <span class="playlist-track-number">{index + 1}.&nbsp;</span>
              <span class="playlist-track-name"
                >{row.track.artist} - {row.track.name}</span
              >
            </td>
            <td class="playlist-track-duration">{row.track.durationAsString}</td
            >
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <!-- Top corners -->
  <div class="sprite playlist-sprite playlist-tl-sprite"></div>

  <div
    class="sprite playlist-sprite playlist-tr-sprite"
    style:--x={size.width}
  ></div>

  <!-- Left/Right -->
  {#each range(1, size.height - 2) as y}
    <div class="sprite playlist-sprite playlist-l-sprite" style:--y={y}></div>
    <div
      class="sprite playlist-sprite playlist-r-sprite"
      style:--y={y}
      style:--x={size.width}
    ></div>
  {/each}

  <!-- Top/Bottom -->
  {#each range(1, size.width - 2) as x}
    <div
      data-tauri-drag-region
      class="sprite playlist-sprite playlist-t-sprite"
      style:--x={x}
    ></div>
    {#if x >= 5 && x < size.width - 6}
      <div
        class="sprite playlist-sprite playlist-b-sprite"
        style:--y={size.height - 1}
        style:--x={x}
      ></div>
    {/if}
  {/each}

  <!-- Title -->
  <div
    data-tauri-drag-region
    class="sprite playlist-sprite playlist-title-sprite"
    style:--x={size.width / 2 - 2}
  ></div>

  <!-- Bottom corners -->
  <div
    class="sprite playlist-sprite playlist-bl-sprite"
    style:--y={size.height}
  ></div>

  <div
    class="sprite playlist-sprite playlist-br-sprite"
    style:--y={size.height - 1}
    style:--x={size.width - 9}
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
      url(assets/px_sans_nouveaux.woff) format("woff");
  }

  .draggable-corner {
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

  /* ------ /TRACKS ------ */

  /* ------ PLAYLIST ------ */
  .playlist-sprite {
    --x: 0;
    --y: 0;
    --sprite-x: calc(var(--x) * 25px);
    --sprite-y: calc(var(--y) * 20px);
  }

  .playlist-tl-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 20px;
  }

  .playlist-l-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 10px;
    height: 29px;
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: 0px -42px;
  }

  .playlist-bl-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 125px;
    height: 38px;
    --y: var(--playlist-h);
    --sprite-y: calc((var(--y) - 1) * 29px - 9px);
    background-position: 0px -72px;
  }

  .playlist-b-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 38px;
    --y: var(--playlist-h);
    --sprite-x: calc(var(--x) * 25px);
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: -179px 0px;
  }

  .playlist-br-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 150px;
    height: 38px;
    --x: var(--playlist-w);
    --y: var(--playlist-h);
    --sprite-x: calc(var(--x) * 25px + 75px);
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: 154px -72px;
  }

  .playlist-r-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 19px;
    height: 29px;
    --x: var(--playlist-w);
    --sprite-x: calc((var(--x) - 1) * 25px + 6px);
    --sprite-y: calc(var(--y) * 29px - 9px);
    background-position: -32px -42px;
  }

  .playlist-tr-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 20px;
    --x: var(--playlist-w);
    --y: 0;
    --sprite-x: calc((var(--x) - 1) * 25px);
    background-position: -153px 0px;
  }

  .playlist-t-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 25px;
    height: 20px;
    --y: 0;
    --sprite-x: calc(var(--x) * 25px);
    background-position: -127px 0px;
  }

  .playlist-title-sprite {
    --sprite-url: url(assets/skins/base-2.91/PLEDIT.BMP);
    width: 100px;
    height: 20px;
    --y: 0;
    --sprite-x: calc(var(--x) * 25px);
    background-position: -26px 0px;
  }

  /* ------ /PLAYLIST ------ */
</style>
