<script>
  import { onDestroy } from "svelte";
  import { glyphs } from "#lib/skin-font.js";
  /**
   * @typedef {Object} Props
   * @property {string} text
   * @property {string | undefined} textOverride
   * @property {boolean} unavailable
   * @property {number} x
   * @property {number} y
   */

  /** @type {Props} */
  let { text, textOverride, unavailable, x, y } = $props();


  const tickerActive = $derived(text.length > 31);

  let xShift = $state(0);
  const SEPARATOR = " *** ";
  let letters = $derived.by(() => {
    const realText = textOverride
      ? textOverride
      : tickerActive
        ? `${text}${SEPARATOR}${text}`
        : text;
    return glyphs(realText);
  });

  // Reset xShift on new text
  $effect(() => {
    void text;
    xShift = 0;
  });

  /** @type {ReturnType<typeof setInterval> | undefined} */
  let ticker;
  $effect(() => {
    clearInterval(ticker);
    if (tickerActive) {
      ticker = setInterval(() => {
        if (!textOverride) {
          xShift = (xShift + 1) % (text.length + SEPARATOR.length);
        }
      }, 220);
    }
    xShift = 0;
  });

  const textWindow = $derived.by(() => {
    const xOffset = textOverride ? 0 : xShift;
    return letters.slice(xOffset, xOffset + Math.min(31, letters.length));
  });
  onDestroy(() => clearInterval(ticker));
</script>

<div
  class="text-container"
  style:--x={x}
  style:--y={y}
  style:opacity={unavailable ? "50%" : "100%"}
>
  {#each textWindow as lut, index}
    <div
      class="sprite letter-sprite"
      style:--letter-idx-row={lut[0]}
      style:--letter-idx-col={lut[1]}
      style:--letter-col={index}
    ></div>
  {/each}
</div>

<style>
  .text-container {
    position: absolute;
    overflow: hidden;
    width: calc(31 * 5px * var(--zoom));
    height: calc(6px * var(--zoom));
    left: calc(var(--x) * var(--zoom) * 1px);
    top: calc(var(--y) * var(--zoom) * 1px);
  }

  .letter-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/TEXT.BMP);
    width: 5px;
    height: 6px;
    background-position: calc(-5px * var(--letter-idx-col))
      calc(-6px * var(--letter-idx-row));
    --sprite-x: calc(var(--letter-col) * 5px);
  }
</style>
