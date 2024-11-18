<script>
    import { REACTIVE_WINDOW_SIZE } from "$lib/common.svelte";
    import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

    /**
     * @type {{children: import("svelte").Snippet}}
     */
    let { children } = $props();
    $effect(() => {
        getCurrentWindow().setSize(
            new LogicalSize(
                REACTIVE_WINDOW_SIZE.width * REACTIVE_WINDOW_SIZE.zoom,
                REACTIVE_WINDOW_SIZE.height * REACTIVE_WINDOW_SIZE.zoom,
            ),
        );
    });
</script>

{@render children()}

<style>
    :global(*) {
        padding: 0;
        margin: 0;
        border: 0;
        image-rendering: pixelated;
        cursor: url(/src/static/assets/skins/base-2.91/MAINMENU.CUR), auto;
    }

    /* Disable scrolling */
    :global(html),
    :global(body) {
        margin: 0;
        height: 100%;
        overflow: hidden;
        background-color: black;
    }

    :global(.sprite) {
        background: var(--sprite-url);
        position: absolute;
        display: inline-block;
        transform-origin: top left;
        transform: scale(var(--zoom));
        left: calc(var(--sprite-x) * var(--zoom));
        top: calc(var(--sprite-y) * var(--zoom));
    }

    :global(.hidden) {
        display: none;
    }

    :global(*:focus) {
        outline: 0 !important;
    }
</style>
