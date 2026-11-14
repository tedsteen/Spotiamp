<!--
  Winamp-style popup menu (the stack of buttons that rises from ADD / REM),
  drawn with the skin's own PLEDIT.BMP sprites.

  Like Winamp, it can be used either by press-drag-release (press the opener,
  slide up to an item, release) or by click-then-click.
-->
<script>
  import { onMount } from "svelte";

  /**
   * @typedef {{
   *   title: string,
   *   sprite: [number, number],
   *   action: () => void,
   * }} PopupItem
   */

  /**
   * @type {{
   *   items: PopupItem[],
   *   x: number,
   *   y: number,
   *   barSprite: [number, number],
   *   openedByPress: boolean,
   *   onclose: () => void,
   * }}
   */
  const { items, x, y, barSprite, openedByPress, onclose } = $props();

  const ITEM_WIDTH = 22;
  const ITEM_HEIGHT = 18;
  /** Horizontal distance from an item's normal sprite to its highlighted one. */
  const PRESSED_OFFSET = 23;

  /** @type {number | undefined} */
  let hovered = $state();
  // The press that opened the menu ends with a mouseup on the bottom item
  // (it sits right on top of the opener). Ignore that one unless the pointer
  // moved away first.
  let ignoreOpeningRelease = openedByPress;

  /**
   * @param {number} index
   */
  function itemTop(index) {
    return y - (items.length - 1 - index) * ITEM_HEIGHT;
  }

  /**
   * @param {number} index
   */
  function onItemMouseUp(index) {
    if (ignoreOpeningRelease) {
      ignoreOpeningRelease = false;
      return;
    }
    onclose();
    items[index].action();
  }

  onMount(() => {
    /** @param {MouseEvent} _ */
    const onWindowMouseUp = (_) => {
      ignoreOpeningRelease = false;
    };
    /** @param {KeyboardEvent} e */
    const onKeyDown = (e) => {
      if (e.key == "Escape") {
        onclose();
      }
    };
    // Registered after the opening mousedown, so the opener's own mouseup is
    // delivered to the item handler first.
    window.addEventListener("mouseup", onWindowMouseUp);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mouseup", onWindowMouseUp);
      window.removeEventListener("keydown", onKeyDown);
    };
  });
</script>

<!-- Clicking anywhere else closes the menu. -->
<div
  class="backdrop"
  role="presentation"
  onmousedown={onclose}
></div>

<div
  class="sprite popup-sprite"
  style:--sprite-x={`${x - 3}px`}
  style:--sprite-y={`${itemTop(0)}px`}
  style:width="3px"
  style:height={`${items.length * ITEM_HEIGHT}px`}
  style:background-position={`-${barSprite[0]}px -${barSprite[1]}px`}
></div>

{#each items as item, index}
  <div
    class="sprite popup-sprite popup-item"
    role="menuitem"
    tabindex="-1"
    title={item.title}
    style:--sprite-x={`${x}px`}
    style:--sprite-y={`${itemTop(index)}px`}
    style:width={`${ITEM_WIDTH}px`}
    style:height={`${ITEM_HEIGHT}px`}
    style:background-position={`-${item.sprite[0] + (hovered === index ? PRESSED_OFFSET : 0)}px -${item.sprite[1]}px`}
    onmouseenter={() => (hovered = index)}
    onmouseleave={() => {
      hovered = undefined;
      ignoreOpeningRelease = false;
    }}
    onmouseup={() => onItemMouseUp(index)}
  ></div>
{/each}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1999;
  }

  .popup-sprite {
    --sprite-url: url(/src/static/assets/skins/base-2.91/PLEDIT.BMP);
    z-index: 2000;
  }
</style>
