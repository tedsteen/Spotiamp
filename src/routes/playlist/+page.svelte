<script>
  import {
    enterExitViewport,
    range,
    handleDrop,
    REACTIVE_WINDOW_SIZE,
  } from "$lib/common.svelte.js";
  import { emitWindowEvent } from "$lib/events.svelte.js";
  import { onMount } from "svelte";
  import { Playlist } from "$lib/playlist.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Window } from "@tauri-apps/api/window";
  import {
    isDocked,
    makeTauriWindowDraggable,
    rectFromPositionAndSize,
    SNAP_DISTANCE,
    snapPosition,
    STICKY_SNAP_DISTANCE,
  } from "$lib/window-docking.svelte.js";

  /** @type {{data: import('./$types').PageData}} */
  const { data: playlistSettings } = $props();

  function applyInitialWindowSize() {
    if (!playlistSettings.window_state.inner_size) {
      return;
    }

    const { width, height } = playlistSettings.window_state.inner_size;
    REACTIVE_WINDOW_SIZE.setSize(width, height);
  }

  function createInitialPlaylist() {
    return new Playlist(playlistSettings.uris);
  }

  applyInitialWindowSize();
  const playlist = createInitialPlaylist();

  /**
   * @param {DocumentEventMap["keydown"]} e
   */
  function preventKeyboardScrolling(e) {
    if (
      ["Space", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].indexOf(
        e.code,
      ) != -1
    ) {
      e.preventDefault();
    }
  }

  onMount(() => {
    const cleanupDropHandler = handleDrop(async (urls) => {
      await playlist.addUrls(urls);
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
  function makeResizable(element) {
    element.onpointerdown = function (event) {
      document.onmousemove = function (event) {
        const pointerX = Math.max(
          Math.ceil(event.clientX / REACTIVE_WINDOW_SIZE.zoom / 25),
          11,
        );
        const pointerY = Math.max(
          Math.ceil(event.clientY / REACTIVE_WINDOW_SIZE.zoom / 29),
          4,
        );

        REACTIVE_WINDOW_SIZE.setSize(pointerX * 25, pointerY * 29);
        invoke("set_playlist_inner_size", {
          width: REACTIVE_WINDOW_SIZE.width,
          height: REACTIVE_WINDOW_SIZE.height,
        });
      };

      document.onmouseup = function () {
        document.onmousemove = null;

        element.releasePointerCapture(event.pointerId);
      };

      element.setPointerCapture(event.pointerId);
    };

    element.onselectstart = () => false;
  }

  /**
   * @param {HTMLElement} element
   */
  function makeWindowDraggable(element) {
    makeTauriWindowDraggable(element, {
      async onStart({ startPosition, windowSize }) {
        const playerWindow = await Window.getByLabel("player");
        if (!playerWindow) {
          return false;
        }
        await emitWindowEvent("playlistWindow", { DragStarted: null });

        const [playerPosition, playerSize] = await Promise.all([
          playerWindow.outerPosition(),
          playerWindow.outerSize(),
        ]);
        const playerRect = rectFromPositionAndSize(playerPosition, playerSize);
        return {
          playerRect,
          playlistSize: windowSize,
          docked: isDocked(
            rectFromPositionAndSize(startPosition, windowSize),
            playerRect,
          ),
        };
      },
      mapPosition(rawPosition, context) {
        const rawRect = {
          ...rawPosition,
          width: context.playlistSize.width,
          height: context.playlistSize.height,
        };
        const snapDistance = context.docked
          ? STICKY_SNAP_DISTANCE
          : SNAP_DISTANCE;
        const snappedPosition = snapPosition(
          rawRect,
          context.playerRect,
          snapDistance,
        );
        context.docked = snappedPosition !== undefined;

        return snappedPosition ?? rawPosition;
      },
      async onEnd() {
        await emitWindowEvent("playlistWindow", { DragEnded: null });
      },
    });
  }
  let scroll = $state(0);
  const PLAYLIST_ROW_HEIGHT = 14.5;
  /**
   * @type {HTMLElement | undefined}
   */
  let scrollElement = $state();
  let wheelDelta = 0;

  function scrollMax() {
    return scrollElement
      ? scrollElement.scrollHeight - scrollElement.clientHeight
      : 0;
  }

  function scrollRowHeight() {
    return PLAYLIST_ROW_HEIGHT * REACTIVE_WINDOW_SIZE.zoom;
  }

  function syncScrollThumb() {
    const max = scrollMax();
    if (scrollElement && max > 0) {
      const value = Math.min(Math.max(0, scrollElement.scrollTop), max);
      scroll = (value / max) * 100;
    } else {
      scroll = 0;
    }
  }

  /**
   * @param {number} row
   */
  function scrollToRow(row) {
    if (!scrollElement) {
      return;
    }

    scrollElement.scrollTop = Math.min(
      Math.max(0, row * scrollRowHeight()),
      scrollMax(),
    );
    syncScrollThumb();
  }

  /**
   * @param {number} offset
   */
  function scrollByRows(offset) {
    if (!scrollElement) {
      return;
    }

    scrollToRow(
      Math.round(scrollElement.scrollTop / scrollRowHeight()) + offset,
    );
  }

  /**
   * @param {WheelEvent} event
   */
  function onWheelScroll(event) {
    let delta = event.deltaY || event.deltaX;
    if (delta === 0) {
      return;
    }

    event.preventDefault();
    if (event.deltaMode == WheelEvent.DOM_DELTA_LINE) {
      delta *= scrollRowHeight();
    } else if (event.deltaMode == WheelEvent.DOM_DELTA_PAGE && scrollElement) {
      delta *= scrollElement.clientHeight;
    }

    wheelDelta += delta;
    const rows =
      wheelDelta > 0
        ? Math.floor(wheelDelta / scrollRowHeight())
        : Math.ceil(wheelDelta / scrollRowHeight());
    if (rows === 0) {
      return;
    }

    scrollByRows(rows);
    wheelDelta -= rows * scrollRowHeight();
  }

  /**
   * @param {Event} event
   */
  function onManualScroll(event) {
    if (scrollElement && event.target instanceof HTMLInputElement) {
      const targetTop = (parseInt(event.target.value, 10) / 100) * scrollMax();
      scrollToRow(Math.round(targetTop / scrollRowHeight()));
    }
  }

  // ------ Drag to reorder ------
  // Winamp-style: the selection shifts by however many rows the pointer has
  // travelled from where the drag started, regardless of which row it's over.
  const EDGE_SCROLL_ZONE = 12;
  const EDGE_SCROLL_INTERVAL_MS = 80;

  /**
   * @typedef {import('$lib/playlist.svelte').TrackRow} Row
   */

  /**
   * @type {{
   *   row: Row,
   *   startY: number,
   *   rowHeight: number,
   *   block: Row[],
   *   remaining: Row[],
   *   baseInsert: number,
   *   originalRows: Row[],
   *   appliedOffset: number,
   *   moved: boolean,
   *   pointerY: number,
   * } | undefined}
   */
  let drag;
  let isDragging = $state(false);
  /** @type {number | undefined} */
  let edgeScrollFrame;
  let lastEdgeScrollAt = 0;

  /**
   * Shift the dragged selection by `offset` rows relative to its start.
   *
   * @param {number} offset
   */
  function applyDragOffset(offset) {
    if (!drag || offset === drag.appliedOffset) {
      return;
    }
    drag.appliedOffset = offset;

    if (offset === 0) {
      // Back at the start: restore the original order verbatim (this also
      // preserves any gaps in a non-contiguous selection).
      playlist.rows = [...drag.originalRows];
    } else {
      drag.moved = true;
      isDragging = true;
      playlist.placeSelection(
        drag.block,
        drag.remaining,
        drag.baseInsert + offset,
      );
    }
  }

  /**
   * Recompute the offset from the current pointer position and apply it.
   */
  function updateDragFromPointer() {
    if (!drag) {
      return;
    }
    const offset = Math.round((drag.pointerY - drag.startY) / drag.rowHeight);
    applyDragOffset(offset);
  }

  /**
   * Continuously scroll while the pointer rests near the top/bottom edge,
   * keeping the offset consistent by shifting the drag origin as we scroll.
   */
  /**
   * @param {number} now
   */
  function edgeScrollTick(now) {
    edgeScrollFrame = undefined;
    if (!drag || !scrollElement) {
      return;
    }

    const rect = scrollElement.getBoundingClientRect();
    let delta = 0;
    if (drag.pointerY < rect.top + EDGE_SCROLL_ZONE) {
      delta = -1;
    } else if (drag.pointerY > rect.bottom - EDGE_SCROLL_ZONE) {
      delta = 1;
    }

    if (delta !== 0 && now - lastEdgeScrollAt >= EDGE_SCROLL_INTERVAL_MS) {
      const before = scrollElement.scrollTop;
      scrollByRows(delta);
      // Move the drag origin by however much we actually scrolled so the
      // pointer-to-row mapping keeps growing while held at the edge.
      drag.startY -= scrollElement.scrollTop - before;
      updateDragFromPointer();
      lastEdgeScrollAt = now;
    }

    if (delta !== 0) {
      edgeScrollFrame = requestAnimationFrame(edgeScrollTick);
    }
  }

  /**
   * @param {MouseEvent} e
   * @param {Row} row
   */
  function onRowMouseDown(e, row) {
    if (e.button !== 0) {
      return;
    }

    const ctrl = e.ctrlKey || e.metaKey;
    const shift = e.shiftKey;
    if (ctrl || shift) {
      playlist.select(row, { ctrl, shift });
      return;
    }

    // Keep an existing multi-selection intact so it can be dragged as a group;
    // a plain click that doesn't turn into a drag collapses to this row on release.
    if (!playlist.selectedRows.includes(row)) {
      playlist.select(row);
    }

    const selected = new Set(playlist.selectedRows);
    const block = playlist.rows.filter((r) => selected.has(r));
    const remaining = playlist.rows.filter((r) => !selected.has(r));
    const topIndex = Math.min(...block.map((r) => playlist.rows.indexOf(r)));
    // Where the block sits among the non-dragged rows at the start.
    const baseInsert = remaining.filter(
      (r) => playlist.rows.indexOf(r) < topIndex,
    ).length;
    const dragElement =
      row.element ??
      (e.currentTarget instanceof HTMLElement ? e.currentTarget : undefined);
    const rowHeight = dragElement?.getBoundingClientRect().height || 1;

    drag = {
      row,
      startY: e.clientY,
      rowHeight,
      block,
      remaining,
      baseInsert,
      originalRows: [...playlist.rows],
      appliedOffset: 0,
      moved: false,
      pointerY: e.clientY,
    };
    lastEdgeScrollAt = 0;
    window.addEventListener("mousemove", onDragMove);
    window.addEventListener("mouseup", onDragEnd);
  }

  /**
   * @param {MouseEvent} e
   */
  function onDragMove(e) {
    if (!drag) {
      return;
    }
    drag.pointerY = e.clientY;
    updateDragFromPointer();

    if (edgeScrollFrame === undefined) {
      edgeScrollFrame = requestAnimationFrame(edgeScrollTick);
    }
  }

  function onDragEnd() {
    window.removeEventListener("mousemove", onDragMove);
    window.removeEventListener("mouseup", onDragEnd);
    if (edgeScrollFrame !== undefined) {
      cancelAnimationFrame(edgeScrollFrame);
      edgeScrollFrame = undefined;
    }

    if (drag && !drag.moved) {
      // A plain click (no drag): collapse the selection to the clicked row.
      playlist.select(drag.row);
    } else if (drag) {
      // The order changed — persist the new arrangement.
      playlist.persist();
    }
    drag = undefined;
    isDragging = false;
    lastEdgeScrollAt = 0;
  }
</script>

<span
  style:--playlist-w={playlist.width}
  style:--playlist-h={playlist.height}
  style:--track-row-height={`${PLAYLIST_ROW_HEIGHT}px`}
>
  <div
    class="tracks-container"
    onkeydown={preventKeyboardScrolling}
    onwheel={onWheelScroll}
    role="scrollbar"
    tabindex="0"
    aria-controls="playlist-tracks"
    aria-valuenow={scroll}
    onscroll={syncScrollThumb}
    bind:this={scrollElement}
  >
    <table id="playlist-tracks" class:dragging={isDragging}>
      <tbody>
        {#each playlist.rows as row, index}
          <tr
            class="playlist-track"
            class:loaded={row.isLoaded()}
            class:selected={row.isSelected()}
            class:unavailable={row.unavailable}
            onmousedown={(e) => onRowMouseDown(e, row)}
            ondblclick={() => row.play()}
            use:enterExitViewport
            bind:this={row.element}
            onenterViewport={row.getOnEnterViewport()}
          >
            <td class="playlist-track-main">
              <span class="playlist-track-number">{index + 1}.&nbsp;</span>
              <span class="playlist-track-name">{row.displayName}</span>
            </td>
            <td class="playlist-track-duration">{row.displayDuration}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <input
      class="sprite scroll-bar"
      type="range"
      bind:value={scroll}
      oninput={onManualScroll}
    />
  </div>

  <!-- Top corners -->
  <div class="sprite playlist-sprite playlist-tl-sprite"></div>

  <div
    class="sprite playlist-sprite playlist-tr-sprite"
    style:--x={playlist.width}
  ></div>

  <!-- Left/Right -->
  {#each range(1, playlist.height - 2) as y}
    <div class="sprite playlist-sprite playlist-l-sprite" style:--y={y}></div>
    <div
      class="sprite playlist-sprite playlist-r-sprite"
      style:--y={y}
      style:--x={playlist.width}
    ></div>
  {/each}

  <!-- Top/Bottom -->
  {#each range(1, playlist.width - 2) as x}
    <div
      class="sprite playlist-sprite playlist-t-sprite"
      style:--x={x}
      use:makeWindowDraggable
    ></div>
    {#if x >= 5 && x < playlist.width - 6}
      <div
        class="sprite playlist-sprite playlist-b-sprite"
        style:--y={playlist.height - 1}
        style:--x={x}
      ></div>
    {/if}
  {/each}

  <!-- Title -->
  <div
    class="sprite playlist-sprite playlist-title-sprite"
    style:--x={playlist.width / 2 - 2}
    use:makeWindowDraggable
  ></div>

  <!-- Bottom corners -->
  <div
    class="sprite playlist-sprite playlist-bl-sprite"
    style:--y={playlist.height}
  ></div>

  <div
    class="sprite playlist-sprite playlist-br-sprite"
    style:--y={playlist.height - 1}
    style:--x={playlist.width - 9}
  ></div>

  <div class="draggable-corner" use:makeResizable></div>
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
    margin-top: calc(20px * var(--zoom));
    margin-left: calc(10px * var(--zoom));
    width: calc((var(--playlist-w) * 25px - 29px) * var(--zoom));
    height: calc(
      (var(--playlist-h) - 2) * 2 * var(--track-row-height) * var(--zoom)
    );
    overflow-x: hidden;
    overflow-y: scroll;
  }

  /* Hide scrollbar for Chrome, Safari and Opera */
  .tracks-container::-webkit-scrollbar {
    display: none;
  }

  /* Hide scrollbar for IE, Edge and Firefox */
  .tracks-container {
    -ms-overflow-style: none; /* IE and Edge */
    scrollbar-width: none; /* Firefox */
  }

  input.scroll-bar {
    cursor: url(/src/static/assets/skins/base-2.91/EQSLID.CUR), default;
    writing-mode: vertical-lr;
    direction: ltr;
    appearance: none;
    --x: var(--playlist-w);
    --y: var(--playlist-h);
    --width: 10px;

    left: calc(((var(--x)) * 25px - var(--width)) * var(--zoom) - 5px);
    top: 20px;

    height: calc(
      (var(--playlist-h) - 2) * 2 * var(--track-row-height) * var(--zoom)
    );
    vertical-align: bottom;
    position: absolute;
    z-index: 1000;
  }

  input.scroll-bar::-webkit-slider-thumb {
    background: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    appearance: none;
    width: 8px;
    height: 18px;
    margin-bottom: 1px;
    background-position: -52px -53px;
  }

  input.scroll-bar::-webkit-slider-thumb:active {
    background-position-x: -61px;
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

  #playlist-tracks.dragging .playlist-track {
    cursor: url(/src/static/assets/skins/base-2.91/TITLEBAR.CUR), grabbing;
  }

  .playlist-track-main {
    /* The max-width:0 + width:100% combo lets the cell take the remaining
       space while still honouring text-overflow within a table layout. */
    max-width: 0;
    width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .playlist-track-number {
    padding-left: calc(3px * var(--zoom));
  }

  .playlist-track-duration {
    padding-right: calc(5px * var(--zoom));
    text-align: right;
    white-space: nowrap;
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
