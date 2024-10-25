<script>
    import { onDestroy } from "svelte";

    /**
     * @typedef {Object} Props
     * @property {string} number
     * @property {string} x
     * @property {string} y
     */

    /** @type {Props} */
    let { number = $bindable(), x, y } = $props();
</script>

<div class="digits-container" style:--x={x} style:--y={y}>
    {#each number as char, index}
        <div
            class="sprite digit-sprite"
            style:--x={index}
            style:--digit={char.charCodeAt(0) - 48}
        ></div>
    {/each}
</div>

<style>
    .digits-container {
        background-color: transparent;
        position: absolute;
        /* overflow: hidden; */
        width: calc(2 * 9px * var(--zoom));
        height: calc(13px * var(--zoom));
        left: calc(var(--x) * var(--zoom) * 1px);
        top: calc(var(--y) * var(--zoom) * 1px);
    }

    .digit-sprite {
        --sprite-url: url(assets/skins/base-2.91/NUMBERS.BMP);
        width: 9px;
        height: 13px;
        left: calc(var(--x) * var(--zoom) * 12px);
        background-position-x: calc(var(--digit) * -9px);
        top: 0px;
    }
</style>
