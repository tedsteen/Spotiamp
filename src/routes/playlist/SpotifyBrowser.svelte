<!--
  Search Spotify / browse your own playlists, drawn over the playlist's track
  area. Double-click (or Enter) adds the highlighted item to the playlist.
  All Spotify requests happen in the backend (see src-tauri/src/browse.rs).
-->
<script>
  import { onMount } from "svelte";
  import {
    getMyPlaylists,
    searchSpotify,
    SpotifyUri,
  } from "#lib/spotify.svelte.js";

  /**
   * @typedef {import('#lib/spotify.svelte.js').BrowseItem} BrowseItem
   * @typedef {import('#lib/spotify.svelte.js').SearchKind | "mine"} Tab
   */

  /**
   * @type {{
   *   playlist: import('#lib/playlist.svelte.js').Playlist,
   *   initialTab: Tab,
   *   onclose: () => void,
   * }}
   */
  const { playlist, initialTab, onclose } = $props();

  /** @type {{ id: Tab, label: string }[]} */
  const TABS = [
    { id: "tracks", label: "SONGS" },
    { id: "albums", label: "ALBUMS" },
    { id: "playlists", label: "PLAYLISTS" },
    { id: "mine", label: "MY LISTS" },
  ];
  const SEARCH_DEBOUNCE_MS = 350;

  /** @type {Tab} starts at `initialTab`, then changed by the tab buttons */
  let tab = $derived(initialTab);
  let query = $state("");
  /** @type {BrowseItem[]} */
  let items = $state([]);
  /** @type {number | null} */
  let nextOffset = $state(null);
  let loading = $state(false);
  let error = $state("");
  let status = $state("");
  let highlighted = $state(-1);
  /** @type {Set<string>} */
  let added = $state(new Set());

  /** @type {HTMLInputElement | undefined} */
  let input = $state();
  /** @type {HTMLElement | undefined} */
  let list = $state();

  /** Bumped on every new request so late responses for old ones are dropped. */
  let requestId = 0;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let debounce;

  async function load() {
    const id = ++requestId;
    clearTimeout(debounce);
    error = "";
    nextOffset = null;
    highlighted = -1;
    if (list) {
      list.scrollTop = 0;
    }

    if (tab != "mine" && !query.trim()) {
      items = [];
      loading = false;
      return;
    }

    loading = true;
    try {
      if (tab == "mine") {
        const result = await getMyPlaylists();
        if (id == requestId) {
          items = result;
        }
      } else {
        const page = await searchSpotify(query.trim(), tab);
        if (id == requestId) {
          items = page.items;
          nextOffset = page.next_offset;
        }
      }
    } catch (e) {
      if (id == requestId) {
        items = [];
        error = String(e);
      }
    } finally {
      if (id == requestId) {
        loading = false;
      }
    }
  }

  async function loadMore() {
    if (loading || nextOffset === null || tab == "mine") {
      return;
    }
    const id = ++requestId;
    loading = true;
    try {
      const page = await searchSpotify(query.trim(), tab, nextOffset);
      if (id == requestId) {
        items = [...items, ...page.items];
        nextOffset = page.next_offset;
      }
    } catch (e) {
      if (id == requestId) {
        error = String(e);
      }
    } finally {
      if (id == requestId) {
        loading = false;
      }
    }
  }

  /**
   * @param {Tab} next
   */
  function selectTab(next) {
    if (tab == next) {
      return;
    }
    tab = next;
    load();
    if (next != "mine") {
      input?.focus();
    }
  }

  function onQueryInput() {
    clearTimeout(debounce);
    // Show "Loading..." straight away rather than a stale "No results".
    loading = query.trim() != "";
    debounce = setTimeout(load, SEARCH_DEBOUNCE_MS);
  }

  /**
   * @param {BrowseItem} item
   */
  async function add(item) {
    status = `Adding ${item.name}...`;
    try {
      await playlist.addUri(SpotifyUri.fromString(item.uri));
      playlist.persist();
      added = new Set(added).add(item.uri);
      status = `Added ${item.name}`;
    } catch (e) {
      status = `Could not add ${item.name}`;
      console.error(`Failed to add ${item.uri}`, e);
    }
  }

  function onListScroll() {
    if (list && list.scrollHeight - list.scrollTop - list.clientHeight < 40) {
      loadMore();
    }
  }

  /**
   * @param {number} index
   */
  function highlight(index) {
    highlighted = Math.max(0, Math.min(items.length - 1, index));
    list
      ?.querySelectorAll("tr")
      [highlighted]?.scrollIntoView({ block: "nearest" });
  }

  /**
   * @param {KeyboardEvent} e
   */
  function onKeyDown(e) {
    if (e.key == "Escape") {
      e.preventDefault();
      onclose();
    } else if (e.key == "ArrowDown" && items.length > 0) {
      e.preventDefault();
      highlight(highlighted + 1);
    } else if (e.key == "ArrowUp" && items.length > 0) {
      e.preventDefault();
      highlight(highlighted - 1);
    } else if (e.key == "Enter") {
      e.preventDefault();
      const item = items[highlighted];
      if (item) {
        add(item);
      } else {
        load();
      }
    }
  }

  onMount(() => {
    if (tab == "mine") {
      load();
    } else {
      input?.focus();
    }
    return () => clearTimeout(debounce);
  });
</script>

<div
  class="browser"
  role="dialog"
  aria-label="Spotify browser"
  tabindex="-1"
  onkeydown={onKeyDown}
>
  <div class="header">
    {#each TABS as t}
      <button class="tab" class:active={tab == t.id} onclick={() => selectTab(t.id)}
        >{t.label}</button
      >
    {/each}
    <button class="close" title="Close" onclick={onclose}>X</button>
  </div>

  {#if tab != "mine"}
    <input
      class="query"
      type="text"
      placeholder="Search Spotify..."
      spellcheck="false"
      bind:this={input}
      bind:value={query}
      oninput={onQueryInput}
    />
  {/if}

  <div class="results" bind:this={list} onscroll={onListScroll}>
    <table>
      <tbody>
        {#each items as item, index (item.uri + index)}
          <tr
            class:highlighted={index == highlighted}
            class:unplayable={!item.playable}
            class:added={added.has(item.uri)}
            title={item.uri}
            onmousedown={() => (highlighted = index)}
            ondblclick={() => add(item)}
          >
            <td class="name">
              {item.name}{#if item.subtitle}<span class="subtitle"
                  >&nbsp;-&nbsp;{item.subtitle}</span
                >{/if}
            </td>
            <td class="info">{item.info}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if loading}
      <div class="message">Loading...</div>
    {:else if error}
      <div class="message error">{error}</div>
    {:else if items.length == 0}
      <div class="message">
        {tab == "mine" ? "No playlists" : query.trim() ? "No results" : "Type to search"}
      </div>
    {/if}
  </div>

  <div class="status">{status || "Double-click or Enter to add"}</div>
</div>

<style>
  .browser {
    position: absolute;
    left: calc(10px * var(--zoom));
    top: calc(20px * var(--zoom));
    width: calc((var(--playlist-w) * 25px - 29px) * var(--zoom));
    height: calc(
      (var(--playlist-h) - 2) * 2 * var(--track-row-height) * var(--zoom)
    );
    z-index: 1500;
    display: flex;
    flex-direction: column;
    background-color: black;
    color: rgb(0, 255, 0);
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(7px * var(--zoom));
    -webkit-font-smoothing: none;
    letter-spacing: calc(0.3px * var(--zoom));
    outline: none;
  }

  /* Same as the playlist: no text selection on (double) click. Kept off the
     search input so it can still be edited normally. WebKit (the macOS
     webview) needs the prefixed property. */
  .header,
  .results,
  .status {
    -webkit-user-select: none;
    user-select: none;
  }

  .header {
    display: flex;
    flex-shrink: 0;
    border-bottom: calc(1px * var(--zoom)) solid rgb(0, 96, 0);
  }

  .header button {
    background: none;
    font: inherit;
    letter-spacing: inherit;
    color: rgb(0, 160, 0);
    padding: calc(2px * var(--zoom)) calc(3px * var(--zoom));
    white-space: nowrap;
  }

  .header button:hover,
  .header .tab.active {
    color: white;
  }

  .header .tab.active {
    background-color: rgb(0, 0, 198);
  }

  .header .close {
    margin-left: auto;
  }

  .query {
    flex-shrink: 0;
    margin: calc(2px * var(--zoom)) calc(3px * var(--zoom));
    padding: calc(1px * var(--zoom)) calc(2px * var(--zoom));
    background-color: black;
    border: calc(1px * var(--zoom)) solid rgb(0, 128, 0);
    color: white;
    font: inherit;
    letter-spacing: inherit;
    cursor: text;
  }

  .query::placeholder {
    color: rgb(0, 128, 0);
  }

  .results {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .results::-webkit-scrollbar {
    display: none;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  tr {
    height: calc(var(--track-row-height) * var(--zoom));
  }

  tr.highlighted {
    background-color: rgb(0, 0, 198);
  }

  tr.added .name {
    color: white;
  }

  tr.unplayable {
    color: rgb(80, 80, 80);
  }

  .name {
    max-width: 0;
    width: 100%;
    padding-left: calc(3px * var(--zoom));
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .subtitle {
    color: rgb(0, 160, 0);
  }

  tr.highlighted .subtitle,
  tr.added .subtitle {
    color: rgb(200, 200, 200);
  }

  .info {
    padding-right: calc(5px * var(--zoom));
    text-align: right;
    white-space: nowrap;
  }

  .message {
    padding: calc(4px * var(--zoom)) calc(3px * var(--zoom));
    color: rgb(0, 160, 0);
  }

  .message.error {
    color: rgb(255, 80, 80);
  }

  .status {
    flex-shrink: 0;
    padding: calc(1px * var(--zoom)) calc(3px * var(--zoom));
    border-top: calc(1px * var(--zoom)) solid rgb(0, 96, 0);
    color: rgb(0, 160, 0);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
