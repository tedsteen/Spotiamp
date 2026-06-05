import { message } from '@tauri-apps/plugin-dialog';

class ReactiveWindowSize {
    width = $state(275.0)
    height = $state(116.0)
    zoom = $state(1.0)

    /**
     * @param {number} width
     * @param {number} height
     */
    setSize(width, height) {
        this.width = width;
        this.height = height;
    }

    /**
     * @param {number} zoom
     */
    setZoom(zoom) {
        this.zoom = zoom;
        document.querySelector('body')?.style.setProperty('--zoom', `${zoom}`);
    }
}
export const REACTIVE_WINDOW_SIZE = new ReactiveWindowSize();

export const enterExitViewportObserver = new IntersectionObserver(
    (entries) => {
        entries.forEach(entry => {
            const eventName = entry.isIntersecting ? 'enterViewport' : 'exitViewport';
            entry.target.dispatchEvent(new CustomEvent(eventName));
        });
    }
);

/**
 * @param {HTMLElement} element 
 */
export function enterExitViewport(element) {
    enterExitViewportObserver.observe(element);

    return {
        destroy() {
            enterExitViewportObserver.unobserve(element);
        }
    }
}

/**
 * @param {number} start 
 * @param {number} end 
 */
export function* range(start, end) {
    for (let i = start; i <= end; i++) {
        yield i;
    }
}

/**
 * @param {Error} e 
 */
export async function handleError(e) {
    await message(`${e}`, { title: 'Spotiamp', kind: 'error' });
}

/**
 * @param {WindowEventMap[keyof WindowEventMap]} ev 
 */
function preventAndStopPropagation(ev) {
    ev.preventDefault();
    ev.stopPropagation();
}

/**
 * @param {(urls: string[]) => void} urlCallback 
 * @returns {() => void} unlisten
 */
export function handleDrop(urlCallback) {
    window.addEventListener("dragenter", preventAndStopPropagation);
    window.addEventListener("dragover", preventAndStopPropagation);
    window.addEventListener("drop", preventAndStopPropagation);

    /**
     * @param {DocumentEventMap["drop"]} ev 
     */
    function documentDropListener(ev) {
        if (ev.dataTransfer) {
            for (const item of ev.dataTransfer.items) {
                if (item.kind === "string" && item.type.match(/^text\/uri-list/)) {
                    item.getAsString((itemText) => {
                        const urls = itemText
                            .split("http")
                            .filter(Boolean) // Remove any empty strings from the beginning
                            .map(url => "http" + url);
                        urlCallback(urls);
                    });
                }
            }
        }

    }
    document.addEventListener("drop", documentDropListener);
    return () => {
        document.removeEventListener("drop", documentDropListener);
        window.removeEventListener("dragenter", preventAndStopPropagation);
        window.removeEventListener("dragover", preventAndStopPropagation);
        window.removeEventListener("drop", preventAndStopPropagation);
    }
}

/**
 * @typedef {{ width: number, height: number }} WindowInnerSize
 */

/**
 * @typedef {{ width: number, height: number }} WindowOuterPosition
 */


/**
 * @typedef {{ inner_size: WindowInnerSize | null, outer_position: WindowOuterPosition | null }} WindowState
 */

/**
 * @typedef {{ volume: number, double_size_active: boolean, show_playlist: boolean, window_state: WindowState }} PlayerSettings
 */

/**
 * @typedef {{ uris: string[], window_state: WindowState }} PlaylistSettings
 */
