import {
  getCurrentWindow,
  PhysicalPosition,
} from "@tauri-apps/api/window";

export const SNAP_DISTANCE = 10;
export const STICKY_SNAP_DISTANCE = 25;

/**
 * @typedef {{ x: number, y: number, width: number, height: number }} WindowRect
 */

/**
 * @typedef {{ x: number, y: number }} WindowPosition
 */

/**
 * @typedef {{
 *   currentWindow: ReturnType<typeof getCurrentWindow>,
 *   startPosition: WindowPosition,
 *   windowSize: { width: number, height: number },
 *   scaleFactor: number,
 *   pointerDownEvent: PointerEvent
 * }} WindowDragStart
 */

/**
 * @param {WindowRect} rect
 */
export function rectRight(rect) {
  return rect.x + rect.width;
}

/**
 * @param {WindowRect} rect
 */
export function rectBottom(rect) {
  return rect.y + rect.height;
}

/**
 * @param {WindowPosition} position
 * @param {{ width: number, height: number }} size
 */
export function rectFromPositionAndSize(position, size) {
  return {
    x: position.x,
    y: position.y,
    width: size.width,
    height: size.height,
  };
}

/**
 * @param {number} aStart
 * @param {number} aEnd
 * @param {number} bStart
 * @param {number} bEnd
 */
function rangesOverlap(aStart, aEnd, bStart, bEnd) {
  return aStart < bEnd && bStart < aEnd;
}

/**
 * @param {number} aStart
 * @param {number} aEnd
 * @param {number} bStart
 * @param {number} bEnd
 */
function rangesTouch(aStart, aEnd, bStart, bEnd) {
  return aStart <= bEnd && bStart <= aEnd;
}

/**
 * @param {{x: number, y: number}} position
 * @param {WindowRect} windowRect
 * @param {WindowRect} otherRect
 * @param {'x' | 'y'} axis
 * @param {number} snapDistance
 */
function snapToGuide(position, windowRect, otherRect, axis, snapDistance) {
  const size = axis == "x" ? windowRect.width : windowRect.height;
  const otherStart = otherRect[axis];
  const otherEnd = axis == "x" ? rectRight(otherRect) : rectBottom(otherRect);
  const ownStart = position[axis];
  const ownEnd = position[axis] + size;

  const candidates = [
    { distance: Math.abs(ownStart - otherStart), value: otherStart },
    { distance: Math.abs(ownStart - otherEnd), value: otherEnd },
    { distance: Math.abs(ownEnd - otherStart), value: otherStart - size },
    { distance: Math.abs(ownEnd - otherEnd), value: otherEnd - size },
  ].sort((a, b) => a.distance - b.distance);

  return candidates[0].distance <= snapDistance
    ? candidates[0].value
    : position[axis];
}

/**
 * @param {WindowRect} windowRect
 * @param {WindowRect} otherRect
 * @param {'x' | 'y'} axis
 * @param {number} snapDistance
 */
function overlapsOrNearGuide(windowRect, otherRect, axis, snapDistance) {
  const start = windowRect[axis];
  const end = axis == "x" ? rectRight(windowRect) : rectBottom(windowRect);
  const otherStart = otherRect[axis];
  const otherEnd = axis == "x" ? rectRight(otherRect) : rectBottom(otherRect);

  return (
    rangesOverlap(start, end, otherStart, otherEnd) ||
    Math.abs(start - otherStart) <= snapDistance ||
    Math.abs(start - otherEnd) <= snapDistance ||
    Math.abs(end - otherStart) <= snapDistance ||
    Math.abs(end - otherEnd) <= snapDistance
  );
}

/**
 * @param {WindowRect} windowRect
 * @param {WindowRect} otherRect
 * @param {number} snapDistance
 */
export function snapPosition(windowRect, otherRect, snapDistance) {
  const candidates = [
    {
      distance: Math.abs(rectRight(windowRect) - otherRect.x),
      snaps: overlapsOrNearGuide(windowRect, otherRect, "y", snapDistance),
      position: {
        x: otherRect.x - windowRect.width,
        y: snapToGuide(
          { x: windowRect.x, y: windowRect.y },
          windowRect,
          otherRect,
          "y",
          snapDistance,
        ),
      },
    },
    {
      distance: Math.abs(windowRect.x - rectRight(otherRect)),
      snaps: overlapsOrNearGuide(windowRect, otherRect, "y", snapDistance),
      position: {
        x: rectRight(otherRect),
        y: snapToGuide(
          { x: windowRect.x, y: windowRect.y },
          windowRect,
          otherRect,
          "y",
          snapDistance,
        ),
      },
    },
    {
      distance: Math.abs(rectBottom(windowRect) - otherRect.y),
      snaps: overlapsOrNearGuide(windowRect, otherRect, "x", snapDistance),
      position: {
        x: snapToGuide(
          { x: windowRect.x, y: windowRect.y },
          windowRect,
          otherRect,
          "x",
          snapDistance,
        ),
        y: otherRect.y - windowRect.height,
      },
    },
    {
      distance: Math.abs(windowRect.y - rectBottom(otherRect)),
      snaps: overlapsOrNearGuide(windowRect, otherRect, "x", snapDistance),
      position: {
        x: snapToGuide(
          { x: windowRect.x, y: windowRect.y },
          windowRect,
          otherRect,
          "x",
          snapDistance,
        ),
        y: rectBottom(otherRect),
      },
    },
  ].sort((a, b) => a.distance - b.distance);

  const candidate = candidates[0];
  return candidate.snaps && candidate.distance <= snapDistance
    ? candidate.position
    : undefined;
}

/**
 * @param {WindowRect[]} rects
 */
export function boundingRect(rects) {
  const x = Math.min(...rects.map((rect) => rect.x));
  const y = Math.min(...rects.map((rect) => rect.y));
  return {
    x,
    y,
    width: Math.max(...rects.map(rectRight)) - x,
    height: Math.max(...rects.map(rectBottom)) - y,
  };
}

/**
 * @param {WindowRect} windowRect
 * @param {WindowRect} boundsRect
 * @param {number} snapDistance
 */
export function snapRectIntoBounds(windowRect, boundsRect, snapDistance) {
  let x = windowRect.x;
  let y = windowRect.y;
  let snapped = false;
  const xCandidates = [
    { distance: Math.abs(windowRect.x - boundsRect.x), value: boundsRect.x },
    {
      distance: Math.abs(rectRight(windowRect) - rectRight(boundsRect)),
      value: rectRight(boundsRect) - windowRect.width,
    },
  ].sort((a, b) => a.distance - b.distance);
  const yCandidates = [
    { distance: Math.abs(windowRect.y - boundsRect.y), value: boundsRect.y },
    {
      distance: Math.abs(rectBottom(windowRect) - rectBottom(boundsRect)),
      value: rectBottom(boundsRect) - windowRect.height,
    },
  ].sort((a, b) => a.distance - b.distance);

  if (xCandidates[0].distance <= snapDistance) {
    x = xCandidates[0].value;
    snapped = true;
  }
  if (yCandidates[0].distance <= snapDistance) {
    y = yCandidates[0].value;
    snapped = true;
  }

  return snapped ? { x, y } : undefined;
}

/**
 * @param {WindowRect} windowRect
 * @param {WindowRect} otherRect
 */
export function isDocked(windowRect, otherRect) {
  const verticallyTouches = rangesTouch(
    windowRect.y,
    rectBottom(windowRect),
    otherRect.y,
    rectBottom(otherRect),
  );
  const horizontallyTouches = rangesTouch(
    windowRect.x,
    rectRight(windowRect),
    otherRect.x,
    rectRight(otherRect),
  );

  return (
    (verticallyTouches && rectRight(windowRect) == otherRect.x) ||
    (verticallyTouches && windowRect.x == rectRight(otherRect)) ||
    (horizontallyTouches && rectBottom(windowRect) == otherRect.y) ||
    (horizontallyTouches && windowRect.y == rectBottom(otherRect))
  );
}

/**
 * @template T
 * @param {HTMLElement} element
 * @param {{
 *   onStart?: (drag: WindowDragStart) => Promise<T | false> | T | false,
 *   mapPosition?: (rawPosition: WindowPosition, context: T, drag: WindowDragStart) => WindowPosition,
 *   onEnd?: (context: T, drag: WindowDragStart) => Promise<void> | void,
 * }} options
 */
export function makeTauriWindowDraggable(element, options = {}) {
  element.onpointerdown = async function (event) {
    event.preventDefault();

    const currentWindow = getCurrentWindow();
    const scaleFactor = await currentWindow.scaleFactor();
    const [startPosition, windowSize] = await Promise.all([
      currentWindow.outerPosition(),
      currentWindow.outerSize(),
    ]);
    const drag = {
      currentWindow,
      startPosition,
      windowSize,
      scaleFactor,
      pointerDownEvent: event,
    };
    const context = await options.onStart?.(drag);
    if (context === false) {
      return;
    }
    const startPointer = {
      x: event.screenX * scaleFactor,
      y: event.screenY * scaleFactor,
    };
    /**
     * @type {WindowPosition | undefined}
     */
    let nextPosition;
    let moving = false;

    async function applyNextPosition() {
      if (moving || !nextPosition) {
        return;
      }

      moving = true;
      const position = nextPosition;
      nextPosition = undefined;
      await currentWindow.setPosition(
        new PhysicalPosition(position.x, position.y),
      );
      moving = false;
      await applyNextPosition();
    }

    async function settlePosition() {
      while (moving || nextPosition) {
        await applyNextPosition();
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }

    /**
     * @param {PointerEvent} event
     */
    element.onpointermove = async function (event) {
      const rawPosition = {
        x: Math.round(
          startPosition.x + event.screenX * scaleFactor - startPointer.x,
        ),
        y: Math.round(
          startPosition.y + event.screenY * scaleFactor - startPointer.y,
        ),
      };

      nextPosition =
        options.mapPosition?.(
          rawPosition,
          /** @type {T} */ (context),
          drag,
        ) ?? rawPosition;
      await applyNextPosition();
    };

    element.onpointerup = async function () {
      element.onpointermove = null;
      element.onpointerup = null;
      element.onpointercancel = null;
      await settlePosition();
      await options.onEnd?.(/** @type {T} */ (context), drag);
      element.releasePointerCapture(event.pointerId);
    };
    element.onpointercancel = element.onpointerup;

    element.setPointerCapture(event.pointerId);
  };

  element.onselectstart = () => false;
}
