<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let error = $state();
  let authUrl = $state();

  /**
   * @param {string} url
   */
  function startLogin(url) {
    error = undefined;
    authUrl = url;
    window.location.href = url;
    setTimeout(() => {
      if (authUrl) {
        error = "Could not reach Spotify. Please check your internet connection.";
      }
    }, 10000);
  }

  function retry() {
    if (authUrl) {
      startLogin(authUrl);
    }
  }

  onMount(async () => {
    try {
      startLogin(await invoke("get_auth_url"));
    } catch (e) {
      error = `Failed to start login: ${e}`;
    }
  });
</script>

{#if error}
  <div class="message">
    <p class="error">{error}</p>
    <button onclick={retry}>Retry</button>
  </div>
{:else}
  <div class="message">
    <div class="progress"></div>
    <p class="loading">Connecting to Spotify...</p>
  </div>
{/if}

<style>
  :global(body) {
    font-family: -apple-system, sans-serif;
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    margin: 0;
    background: #1e1e1e;
    color: #fff;
  }

  .message {
    text-align: center;
    padding: 2rem;
  }

  .loading {
    color: #aaa;
  }

  .error {
    color: #e74c3c;
  }

  button {
    margin-top: 1rem;
    padding: 0.5rem 1.5rem;
    font-size: 1rem;
    background: #1db954;
    color: #fff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  button:hover {
    background: #1ed760;
  }

  .progress {
    border: 3px solid #333;
    border-top: 3px solid #1db954;
    border-radius: 50%;
    width: 30px;
    height: 30px;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
