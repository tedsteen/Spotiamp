<!--
  "ADD URL": paste an open.spotify.com link or a spotify: URI (track, album or
  playlist) and add it to the playlist.
-->
<script>
  import { onMount } from "svelte";
  import { SpotifyUri } from "#lib/spotify.svelte.js";

  /**
   * @type {{
   *   playlist: import('#lib/playlist.svelte.js').Playlist,
   *   onclose: () => void,
   * }}
   */
  const { playlist, onclose } = $props();

  let value = $state("");
  let error = $state("");
  let adding = $state(false);
  /** @type {HTMLInputElement | undefined} */
  let input = $state();

  async function submit() {
    if (adding || !value.trim()) {
      return;
    }
    let uri;
    try {
      uri = SpotifyUri.parse(value);
    } catch {
      error = "Not a Spotify track, album or playlist link";
      return;
    }
    adding = true;
    error = "";
    try {
      await playlist.addUri(uri);
      playlist.persist();
      onclose();
    } catch (e) {
      error = `Could not add it (${e})`;
    } finally {
      adding = false;
    }
  }

  /**
   * @param {KeyboardEvent} e
   */
  function onKeyDown(e) {
    if (e.key == "Enter") {
      e.preventDefault();
      submit();
    } else if (e.key == "Escape") {
      e.preventDefault();
      onclose();
    }
  }

  onMount(() => input?.focus());
</script>

<div class="backdrop" role="presentation" onmousedown={onclose}></div>
<div class="dialog" role="dialog" aria-label="Add URL">
  <div class="title">
    <span>ADD URL</span>
    <button title="Close" onclick={onclose}>X</button>
  </div>
  <input
    type="text"
    spellcheck="false"
    placeholder="https://open.spotify.com/... or spotify:..."
    bind:this={input}
    bind:value
    oninput={() => (error = "")}
    onkeydown={onKeyDown}
  />
  <div class="hint" class:error>
    {adding ? "Adding..." : error || "Track, album or playlist. Enter to add."}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1999;
  }

  .dialog {
    position: absolute;
    left: calc(10px * var(--zoom));
    top: calc(20px * var(--zoom));
    width: calc((var(--playlist-w) * 25px - 29px) * var(--zoom));
    z-index: 2000;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: calc(2px * var(--zoom));
    padding: calc(2px * var(--zoom)) calc(3px * var(--zoom));
    background-color: black;
    border-bottom: calc(1px * var(--zoom)) solid rgb(0, 96, 0);
    color: rgb(0, 255, 0);
    font-family: "px sans nouveaux", sans-serif;
    font-size: calc(7px * var(--zoom));
    -webkit-font-smoothing: none;
    letter-spacing: calc(0.3px * var(--zoom));
  }

  .title,
  .hint {
    -webkit-user-select: none;
    user-select: none;
  }

  .title {
    display: flex;
    justify-content: space-between;
    color: white;
  }

  .title button {
    background: none;
    font: inherit;
    color: rgb(0, 160, 0);
  }

  .title button:hover {
    color: white;
  }

  input {
    padding: calc(1px * var(--zoom)) calc(2px * var(--zoom));
    background-color: black;
    border: calc(1px * var(--zoom)) solid rgb(0, 128, 0);
    color: white;
    font: inherit;
    letter-spacing: inherit;
    cursor: text;
  }

  input::placeholder {
    color: rgb(0, 128, 0);
  }

  .hint {
    color: rgb(0, 160, 0);
  }

  .hint.error {
    color: rgb(255, 80, 80);
  }
</style>
