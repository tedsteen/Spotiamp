import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { SpotifyTrack } from './spotifyTrack';
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit, } from "@tauri-apps/api/event";

export const ORIGINAL_ZOOM = window.innerWidth / 275.0;

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
 * @callback urlCallback
 * @param {string} url
 */

/**
 * @param {urlCallback} urlCallback 
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

const spotifyUrlRe = /https:\/\/open.spotify.com\/(.*)\/(.{22})/;

/**
 * @param {string} url
 * @returns {string}
 */
export function spotifyUrlToUri(url) {
    const matches = spotifyUrlRe.exec(url);
    if (matches) {
        return `spotify:${matches[1]}:${matches[2]}`;
    }
    throw `${url} does not match a spotify URL`;
}

/**
 * @param {import('$lib/spotifyTrack').Uri} uri
 * @returns {Promise<SpotifyTrack>}
 */
async function getTrack(uri) {
    const trackData = await invoke("get_track", { uri });
    return new SpotifyTrack(trackData.artist, trackData.name, trackData.duration, uri);
}

/**
 * @param {string} url 
 * @returns 
 */
export async function spotifyUrlToTrack(url) {
    return await getTrack(spotifyUrlToUri(url))
}

/**
 * @typedef { {playlistWindow: {event: {Ready: null}}, playerWindow: {event: {Ready: null, ChangeVolume: number, NextPressed: null, PreviousPressed: null }}, player: { event: { 'TrackChanged': {track_id: number, track_uri: string, artist: string, name: string, duration: number}, 'Paused': { id: number, position_ms: number}, 'Playing': { id: number, position_ms: number}, 'Stopped': {id: number}, 'EndOfTrack': {id: number}, 'PositionCorrection': { id: number, position_ms: number}, 'Seeked': { id: number, position_ms: number}} }} } WindowEventTypes
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
