<!-- Static text in Winamp's bitmap font (TEXT.BMP), positioned in skin pixels. -->
<script>
  import { glyphs } from "#lib/skin-font.js";

  /**
   * @typedef {Object} Props
   * @property {string} text
   * @property {number} x
   * @property {number} y
   * @property {number} maxChars text beyond this is cut off
   */

  /** @type {Props} */
  let { text, x, y, maxChars } = $props();

  const letters = $derived(glyphs(text).slice(0, maxChars));
</script>

<div class="text-container" style:--x={x} style:--y={y} style:--max-chars={maxChars}>
  {#each letters as [row, col], index}
    <div
      class="sprite letter-sprite"
      style:--letter-row={row}
      style:--letter-col={col}
      style:--index={index}
    ></div>
  {/each}
</div>

<style>
  .text-container {
    position: absolute;
    overflow: hidden;
    width: calc(var(--max-chars) * 5px * var(--zoom));
    height: calc(6px * var(--zoom));
    left: calc(var(--x) * var(--zoom) * 1px);
    top: calc(var(--y) * var(--zoom) * 1px);
    pointer-events: none;
  }

  .letter-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/TEXT.BMP);
    width: 5px;
    height: 6px;
    background-position: calc(-5px * var(--letter-col))
      calc(-6px * var(--letter-row));
    --sprite-x: calc(var(--index) * 5px);
    --sprite-y: 0px;
  }
</style>
