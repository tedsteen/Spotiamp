<script>
    import { ORIGINAL_ZOOM, setZoom } from "$lib/common.js";
    setZoom(ORIGINAL_ZOOM);
</script>

<slot />

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
