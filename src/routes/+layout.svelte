<script>
  import { REACTIVE_WINDOW_SIZE } from "$lib/common.svelte";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import "./global.css";
  /**
   * @type {{children: import("svelte").Snippet}}
   */
  let { children } = $props();
  $effect(() => {
    getCurrentWindow().setSize(
      new LogicalSize(
        REACTIVE_WINDOW_SIZE.width * REACTIVE_WINDOW_SIZE.zoom,
        REACTIVE_WINDOW_SIZE.height * REACTIVE_WINDOW_SIZE.zoom
      )
    );
  });
</script>

{@render children()}
