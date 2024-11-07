import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { SpotifyTrack } from './spotifyTrack';

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
 */
export function handleDrop(urlCallback) {
    window.addEventListener("dragenter", preventAndStopPropagation);
    window.addEventListener("dragover", preventAndStopPropagation);
    window.addEventListener("drop", preventAndStopPropagation);

    document.addEventListener("drop", (ev) => {

        if (ev.dataTransfer) {
            for (const item of ev.dataTransfer.items) {
                if (item.kind === "string" && item.type.match(/^text\/uri-list/)) {
                    item.getAsString((url) => {
                        urlCallback(url);
                    });
                }
            }
        }
    });
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
 * @callback playerEventCallback
 * @param { { payload: { 'TrackChanged': {track_id: number, track_uri: string, artist: string, name: string, duration: number}, 'Paused': { id: number, position_ms: number}, 'Playing': { id: number, position_ms: number}, 'Stopped': {id: number}, 'EndOfTrack': {id: number}, 'PositionCorrection': { id: number, position_ms: number}, 'Seeked': { id: number, position_ms: number}} }} event
 */

import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * @param {playerEventCallback} callback 
 */
export function subscribeToPlayerEvents(callback) {
    getCurrentWindow().listen("player", callback);
}