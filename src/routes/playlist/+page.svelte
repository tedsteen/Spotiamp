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
  import { SpotifyUri } from "$lib/spotify.svelte.js";
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
    fetchToken();
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

  let token = $state("");
  /** @type {"add" | "rem" | null} */
  let activeMenu = $state(null);
  let showSearchModal = $state(false);
  let searchQuery = $state("");
  let searchType = $state("track"); // 'track' | 'playlist' | 'my-playlists'
  /** @type {any[]} */
  let searchResults = $state([]);
  let isSearching = $state(false);
  let showUrlModal = $state(false);
  let urlInput = $state("");
  let errorMessage = $state("");

  async function fetchToken() {
    try {
      token = await invoke("get_spotify_access_token");
    } catch (e) {
      console.error("Failed to get Spotify access token:", e);
    }
  }

  function openSearchModal() {
    showSearchModal = true;
    searchQuery = "";
    searchResults = [];
    searchType = "track";
    errorMessage = "";
  }

  function closeSearchModal() {
    showSearchModal = false;
  }

  async function performSearch() {
    errorMessage = "";
    if (!token) {
      await fetchToken();
    }
    if (!token) {
      errorMessage = "Spotify access token not available.";
      console.error(errorMessage);
      return;
    }

    if (searchType === 'my-playlists') {
      await fetchMyPlaylists();
      return;
    }

    if (!searchQuery.trim()) {
      searchResults = [];
      return;
    }

    isSearching = true;
    try {
      const q = encodeURIComponent(searchQuery);
      const url = `https://api.spotify.com/v1/search?q=${q}&type=${searchType}&limit=20`;
      const response = await fetch(url, {
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });
      if (!response.ok) {
        if (response.status === 401) {
          await fetchToken();
          const retryResponse = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` }
          });
          if (!retryResponse.ok) {
            throw new Error(`Spotify API error after retry: ${retryResponse.status}`);
          }
          const data = await retryResponse.json();
          searchResults = parseResults(data);
          return;
        }
        throw new Error(`Spotify API error: ${response.status}`);
      }
      const data = await response.json();
      searchResults = parseResults(data);
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : String(e);
    } finally {
      isSearching = false;
    }
  }

  async function fetchMyPlaylists() {
    errorMessage = "";
    isSearching = true;
    try {
      const url = `https://api.spotify.com/v1/me/playlists?limit=50`;
      const response = await fetch(url, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (!response.ok) {
        throw new Error(`Spotify API error: ${response.status}`);
      }
      const data = await response.json();
      searchResults = (data.items || [])
        .filter((/** @type {any} */ item) => item !== null && item !== undefined)
        .map((/** @type {any} */ item) => ({
          id: item.id || '',
          name: item.name || 'Unnamed Playlist',
          type: 'playlist',
          uri: item.uri || '',
          owner: item.owner?.display_name || 'Unknown',
          trackCount: item.tracks?.total || 0
        }));
    } catch (e) {
      console.error(e);
      errorMessage = e instanceof Error ? e.message : String(e);
    } finally {
      isSearching = false;
    }
  }

  /**
   * @param {any} data
   */
  function parseResults(data) {
    if (searchType === 'track') {
      return (data.tracks?.items || [])
        .filter((/** @type {any} */ item) => item !== null && item !== undefined)
        .map((/** @type {any} */ item) => ({
          id: item.id || '',
          name: item.name || 'Unknown Song',
          type: 'track',
          uri: item.uri || '',
          artist: (item.artists || []).map((/** @type {any} */ a) => a?.name || '').filter(Boolean).join(', ') || 'Unknown Artist',
          duration: item.duration_ms || 0
        }));
    } else {
      return (data.playlists?.items || [])
        .filter((/** @type {any} */ item) => item !== null && item !== undefined)
        .map((/** @type {any} */ item) => ({
          id: item.id || '',
          name: item.name || 'Unnamed Playlist',
          type: 'playlist',
          uri: item.uri || '',
          owner: item.owner?.display_name || 'Unknown',
          trackCount: item.tracks?.total || 0
        }));
    }
  }

  /**
   * @param {any} item
   */
  async function addResultToPlaylist(item) {
    try {
      await playlist.addUri(SpotifyUri.fromString(item.uri));
      playlist.persist();
    } catch (e) {
      console.error("Failed to add item to playlist:", e);
    }
  }

  function submitUrlModal() {
    const url = urlInput.trim();
    if (url) {
      try {
        let uri;
        if (url.startsWith("spotify:")) {
          uri = SpotifyUri.fromString(url);
        } else {
          const urlWithoutQuery = url.split('?')[0];
          uri = SpotifyUri.fromUrl(urlWithoutQuery);
        }
        playlist.addUri(uri).then(() => {
          playlist.persist();
        }).catch((e) => {
          console.error("Failed to add URI:", e);
        });
        showUrlModal = false;
        urlInput = "";
      } catch (e) {
        alert("Invalid Spotify URL or URI");
      }
    }
  }

  /**
   * @param {any} e
   */
  function stopKeyPropagation(e) {
    e.stopPropagation();
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

  <!-- Overlay interactive buttons for ADD and REM positioned via calc directly for robust hit-areas -->
  <button class="playlist-action-btn add-btn" onclick={(e) => { e.stopPropagation(); activeMenu = activeMenu === 'add' ? null : 'add'; }} aria-label="Add tracks"></button>
  <button class="playlist-action-btn rem-btn" onclick={(e) => { e.stopPropagation(); activeMenu = activeMenu === 'rem' ? null : 'rem'; }} aria-label="Remove tracks"></button>

  <!-- Winamp Dropdown menus -->
  {#if activeMenu === 'add'}
    <button class="menu-backdrop" onclick={() => activeMenu = null} aria-label="Close menu"></button>
    <div class="winamp-menu add-menu" style="left: calc(10px * var(--zoom)); bottom: calc(28px * var(--zoom));">
      <button onclick={() => { activeMenu = null; openSearchModal(); }}>Search Spotify</button>
      <button onclick={() => { activeMenu = null; showUrlModal = true; urlInput = ""; }}>Add URL</button>
    </div>
  {/if}

  {#if activeMenu === 'rem'}
    <button class="menu-backdrop" onclick={() => activeMenu = null} aria-label="Close menu"></button>
    <div class="winamp-menu rem-menu" style="left: calc(39px * var(--zoom)); bottom: calc(28px * var(--zoom));">
      <button onclick={() => { activeMenu = null; playlist.removeSelected(); }}>Remove Selected</button>
      <button onclick={() => { activeMenu = null; playlist.clear(); }}>Clear Playlist</button>
    </div>
  {/if}

  <!-- Add URL Modal -->
  {#if showUrlModal}
    <button class="menu-backdrop" onclick={() => showUrlModal = false} aria-label="Close modal"></button>
    <div class="winamp-menu winamp-dialog url-modal-container" style="left: calc(10px * var(--zoom)); width: calc((var(--playlist-w) * 25px - 29px) * var(--zoom));">
      <div class="search-header">
        <span class="search-title">ADD URL OR URI</span>
        <button class="search-close-btn" onclick={() => showUrlModal = false}>X</button>
      </div>
      <div class="search-input-wrapper">
        <input
          class="search-input"
          type="text"
          bind:value={urlInput}
          placeholder="https://open.spotify.com/..."
          onkeydown={(e) => {
            stopKeyPropagation(e);
            if (e.key === 'Enter') {
              submitUrlModal();
            }
          }}
        />
        <button class="search-btn" onclick={submitUrlModal}>OK</button>
      </div>
    </div>
  {/if}

  <!-- Spotify Search Panel Overlay -->
  {#if showSearchModal}
    <div class="search-container">
      <div class="search-header">
        <span class="search-title">SPOTIFY SEARCH</span>
        <button class="search-close-btn" onclick={closeSearchModal}>X</button>
      </div>
      <div class="search-tabs">
        <button class:active={searchType === 'track'} onclick={() => { searchType = 'track'; errorMessage = ""; performSearch(); }}>SONGS</button>
        <button class:active={searchType === 'playlist'} onclick={() => { searchType = 'playlist'; errorMessage = ""; performSearch(); }}>PLAYLISTS</button>
        <button class:active={searchType === 'my-playlists'} onclick={() => { searchType = 'my-playlists'; errorMessage = ""; performSearch(); }}>MY PLAYLISTS</button>
      </div>
      {#if searchType !== 'my-playlists'}
        <div class="search-input-wrapper">
          <input
            class="search-input"
            type="text"
            bind:value={searchQuery}
            placeholder="Search Spotify..."
            onkeydown={(e) => {
              stopKeyPropagation(e);
              if (e.key === 'Enter') {
                performSearch();
              }
            }}
          />
          <button class="search-btn" onclick={performSearch}>FIND</button>
        </div>
      {/if}

      <div class="search-results">
        {#if isSearching}
          <div class="search-loading">Searching...</div>
        {:else if errorMessage}
          <div class="search-error" style="color: #ff5555; padding: 10px; font-family: 'px sans nouveaux', monospace; font-size: calc(9px * var(--zoom)); text-align: center; line-height: 1.4; word-break: break-all;">
            {errorMessage}
          </div>
        {:else if searchResults.length === 0}
          <div class="search-empty">No results found</div>
        {:else}
          <table>
            <tbody>
              {#each searchResults as item}
                <tr class="search-item" ondblclick={() => addResultToPlaylist(item)}>
                  <td class="search-item-main">
                    {#if item.type === 'track'}
                      <span>{item.name}</span>
                      <span style="color: #888888; font-size: calc(5.5px * var(--zoom));"> - {item.artist}</span>
                    {:else}
                      <span>{item.name}</span>
                      <span style="color: #888888; font-size: calc(5.5px * var(--zoom));"> by {item.owner}</span>
                    {/if}
                  </td>
                  <td class="search-item-info">
                    {#if item.type === 'track'}
                      {Math.floor(item.duration / 60000)}:{String(Math.floor((item.duration % 60000) / 1000)).padStart(2, '0')}
                    {:else}
                      {item.trackCount} tracks
                    {/if}
                  </td>
                  <td class="search-item-action">
                    <button class="add-track-btn" onclick={() => addResultToPlaylist(item)}>ADD</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>
  {/if}
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

  /* ------ BOTTOM BUTTONS OVERLAY ------ */
  .playlist-action-btn {
    position: absolute;
    top: calc(((var(--playlist-h) * 29px) - 30px) * var(--zoom));
    width: calc(22px * var(--zoom));
    height: calc(18px * var(--zoom));
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
    z-index: 1005;
  }
  .playlist-action-btn.add-btn {
    left: calc(14px * var(--zoom));
  }
  .playlist-action-btn.rem-btn {
    left: calc(43px * var(--zoom));
  }

  /* ------ WINAMP CONTEXT MENUS ------ */
  button.menu-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    z-index: 1999;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
  }

  .winamp-menu {
    position: absolute;
    background-color: #2b2b2b;
    border: 2px solid;
    border-color: #555555 #111111 #111111 #555555;
    box-shadow: 1px 1px 0px 0px #000000;
    z-index: 2000;
    padding: 1px;
    display: flex;
    flex-direction: column;
    min-width: calc(100px * var(--zoom));
    box-sizing: border-box;
  }

  .winamp-menu button {
    background: transparent;
    border: none;
    color: #ffffff;
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(7px * var(--zoom));
    text-align: left;
    padding: calc(3px * var(--zoom)) calc(6px * var(--zoom));
    width: 100%;
    box-sizing: border-box;
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
  }

  .winamp-menu button:hover {
    background-color: rgb(0, 0, 198);
    color: #ffffff;
  }

  /* ------ SPOTIFY SEARCH MODAL ------ */
  .search-container {
    position: absolute;
    left: calc(10px * var(--zoom));
    top: calc(20px * var(--zoom));
    width: calc((var(--playlist-w) * 25px - 29px) * var(--zoom));
    height: calc((var(--playlist-h) - 2) * 2 * 14.5px * var(--zoom));
    background-color: #000000;
    z-index: 1001;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    border: 2px solid;
    border-color: #555555 #111111 #111111 #555555;
    font-family: "px sans nouveaux", sans-serif;
    color: rgb(0, 255, 0);
    font-size: calc(7px * var(--zoom));
  }

  .search-header {
    background-color: #2b2b2b;
    padding: calc(3px * var(--zoom)) calc(6px * var(--zoom));
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid #111111;
    color: #ffffff;
  }

  .search-title {
    font-weight: bold;
    letter-spacing: calc(0.5px * var(--zoom));
  }

  .search-close-btn {
    background: transparent;
    border: none;
    color: #888888;
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(7px * var(--zoom));
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
    padding: 0 calc(2px * var(--zoom));
  }
  .search-close-btn:hover {
    color: #ff0000;
  }

  .search-tabs {
    display: flex;
    background-color: #1a1a1a;
    border-bottom: 2px solid #111111;
  }

  .search-tabs button {
    flex: 1;
    background: transparent;
    border: none;
    border-right: 2px solid #111111;
    color: #888888;
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(6px * var(--zoom));
    padding: calc(4px * var(--zoom)) 0;
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
    text-align: center;
  }

  .search-tabs button:last-child {
    border-right: none;
  }

  .search-tabs button.active {
    background-color: #000000;
    color: rgb(0, 255, 0);
  }

  .search-input-wrapper {
    display: flex;
    padding: calc(4px * var(--zoom));
    gap: calc(4px * var(--zoom));
    background-color: #2b2b2b;
    border-bottom: 2px solid #111111;
  }

  .search-input {
    flex: 1;
    background-color: #000000;
    border: 2px solid;
    border-color: #111111 #555555 #555555 #111111;
    color: rgb(0, 255, 0);
    font-family: sans-serif;
    font-size: calc(8px * var(--zoom));
    padding: calc(2px * var(--zoom)) calc(4px * var(--zoom));
    outline: none;
  }

  .search-btn {
    background-color: #2b2b2b;
    border: 2px solid;
    border-color: #555555 #111111 #111111 #555555;
    color: #ffffff;
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(6.5px * var(--zoom));
    padding: 0 calc(8px * var(--zoom));
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
  }
  .search-btn:active {
    border-color: #111111 #555555 #555555 #111111;
  }

  .search-results {
    flex: 1;
    overflow-y: scroll;
    overflow-x: hidden;
  }

  .search-results::-webkit-scrollbar {
    display: none;
  }
  .search-results {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }

  .search-results table {
    width: 100%;
    border-collapse: collapse;
  }

  .search-item {
    height: calc(14.5px * var(--zoom));
    border-bottom: 1px solid #111111;
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
  }

  .search-item:hover {
    background-color: rgb(0, 0, 198);
    color: #ffffff !important;
  }

  .search-item-main {
    padding-left: calc(4px * var(--zoom));
    max-width: 0;
    width: 70%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: calc(6.5px * var(--zoom));
  }

  .search-item-info {
    text-align: right;
    padding-right: calc(4px * var(--zoom));
    white-space: nowrap;
    color: #888888;
    font-size: calc(6.5px * var(--zoom));
  }
  .search-item:hover .search-item-info {
    color: #ffffff;
  }

  .search-item-action {
    width: calc(32px * var(--zoom));
    text-align: center;
    padding-right: calc(2px * var(--zoom));
  }

  .add-track-btn {
    background-color: #2b2b2b;
    border: 2px solid;
    border-color: #555555 #111111 #111111 #555555;
    color: rgb(0, 255, 0);
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(5px * var(--zoom));
    padding: calc(1px * var(--zoom)) calc(3px * var(--zoom));
    cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), pointer;
    vertical-align: middle;
  }
  .add-track-btn:active {
    border-color: #111111 #555555 #555555 #111111;
  }
  .search-item:hover .add-track-btn {
    background-color: #ffffff;
    color: rgb(0, 0, 198);
    border-color: #ffffff;
  }

  .search-loading, .search-empty {
    padding: calc(12px * var(--zoom));
    text-align: center;
    color: #888888;
  }

  .url-modal-container {
    height: auto !important;
    top: calc(20px * var(--zoom));
    font-family: "px sans nouveaux", sans-serif;
    color: rgb(0, 255, 0);
    font-size: calc(7px * var(--zoom));
  }
</style>
