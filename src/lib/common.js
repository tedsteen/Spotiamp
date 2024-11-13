import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit, } from "@tauri-apps/api/event";

export const ORIGINAL_ZOOM = window.innerWidth / 275.0;
export class MMSS {
    /**
     * @param {number} m 
     * @param {number} s 
     */
    constructor(m, s) {
        this.m = m;
        this.s = s;
    }
}
/**
 * @param {number} durationInMs
 * @returns {MMSS}
 */
export function durationToMMSS(durationInMs) {
    durationInMs = Math.floor(durationInMs / 1000);
    const minutes = Math.floor((durationInMs) / 60);
    const seconds = durationInMs - (minutes * 60);
    return new MMSS(minutes, seconds);
}
/**
 * @param {number} durationInMs
 * @returns {string}
 */
export function durationToString(durationInMs) {
    const hhmmss = durationToMMSS(durationInMs);

    let timeString = hhmmss.m.toString().padStart(1, '0') + ':' +
        hhmmss.s.toString().padStart(2, '0');
    return timeString;
}

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

export class SpotifyUri {
    /**
     * @param {"track" | "playlist"} type 
     * @param {string} id 
     */
    constructor(type, id) {
        this.type = type;
        this.id = id;
        this.asString = `spotify:${this.type}:${this.id}`;
    }
}

const spotifyUriRe = /spotify:(.*):(.{22})/;
/**
 * @param {string} uriAsString 
 */
SpotifyUri.fromString = function (uriAsString) {
    const matches = spotifyUriRe.exec(uriAsString);
    if (matches?.length == 3) {
        const type = matches[1], id = matches[2];
        if (type == "track" || type == "playlist") {
            return new SpotifyUri(type, id);
        } else {
            throw `Only track and playlist types allowed as spotify URIs`;
        }
    }

    throw `${uriAsString} does not match a spotify URI`;
}

const spotifyUrlRe = /https:\/\/open.spotify.com\/(.*)\/(.{22})/;
/**
 * @param {string} url
 * @returns {SpotifyUri}
 */
SpotifyUri.fromUrl = function (url) {
    const matches = spotifyUrlRe.exec(url);
    if (matches?.length == 3) {
        const type = matches[1], id = matches[2];
        if (type == "track" || type == "playlist") {
            return new SpotifyUri(type, id);
        } else {
            throw `Only track and playlist types allowed as spotify URLs`;
        }
    }

    throw `${url} does not match a spotify URL`;
}

export class SpotifyTrack {
    /**
    * @param {string} artist
    * @param {string} name
    * @param {number} durationInMs
    * @param {SpotifyUri} uri
    * @param {boolean} unavailable
    */
    constructor(artist, name, durationInMs, uri, unavailable) {
        this.name = name;
        this.artist = artist;
        this.durationInMs = durationInMs
        this.displayDuration = durationToString(durationInMs);
        this.displayName = `${this.artist} - ${this.name}`;
        this.uri = uri;
        this.unavailable = unavailable;
    }
}

/**
 * @param {SpotifyUri} uri
 * @returns {Promise<SpotifyTrack>}
 */
SpotifyTrack.loadFromUri = async function (uri) {
    /**
     * @type {{artist: string, name: string, duration: number, uri: string, unavailable: boolean}}
     */
    const trackData = await invoke("get_track_metadata", { uri: uri.asString });
    return new SpotifyTrack(trackData.artist, trackData.name, trackData.duration, uri, trackData.unavailable);
}


/**
 * @param {number} zoom 
 */
export function setZoom(zoom) {
    document.querySelector('body')?.style.setProperty('--zoom', `${zoom}`);
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
 * @param {(url: string) => void} urlCallback 
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
                    item.getAsString((url) => {
                        urlCallback(url);
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
 * @typedef { {playlistWindow: {event: {Ready: null, PlayRequsted: null, TrackLoaded: SpotifyTrack, EndReached: null}}, playerWindow: {event: {Ready: null, NextPressed: null, PreviousPressed: null }}, player: { event: { 'TrackChanged': {uri: string}, 'Paused': { id: number, position_ms: number}, 'Playing': { id: number, position_ms: number}, 'Stopped': {id: number}, 'EndOfTrack': {id: number}, 'PositionCorrection': { id: number, position_ms: number}, 'Seeked': { id: number, position_ms: number}} }} } WindowEventTypes
 */

/**
 * @template {keyof WindowEventTypes} T
 * @template {keyof WindowEventTypes[T]["event"]} T2
 * @param {T} key
 * @param {{[P in T2]: WindowEventTypes[T]["event"][P]}} event
 */
export async function emitWindowEvent(key, event) {
    await emit(key, event)
}

const CURRENT_WINDOW = getCurrentWindow();
/**
 * @template {keyof WindowEventTypes} T
 * @param {T} key
 * @param {(event: WindowEventTypes[T]["event"]) => void} callback 
 */
export async function subscribeToWindowEvent(key, callback) {
    return await CURRENT_WINDOW.listen(key, (event) => { callback(event.payload) });
}
