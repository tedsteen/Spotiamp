import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

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
 * @typedef {(string)} Uri
 */

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
 * @param {number} durationInMs
 * @returns {string}
 */
function durationToHHMMSS(durationInMs) {
    durationInMs = Math.floor(durationInMs / 1000);
    const hours = Math.floor(durationInMs / 3600);
    const minutes = Math.floor((durationInMs - (hours * 3600)) / 60);
    const seconds = durationInMs - (hours * 3600) - (minutes * 60);

    let timeString = hours > 0 ? hours.toString().padStart(1, '0') + ':' : "";
    timeString += minutes.toString().padStart(1, '0') + ':' +
        seconds.toString().padStart(2, '0');
    return timeString;
}

export class SpotifyTrack {
    /**
    * @param {string} artist
    * @param {string} name
    * @param {number} durationInMs
    * @param {Uri} uri
    */
    constructor(artist, name, durationInMs, uri) {
        this.name = name;
        this.artist = artist;
        this.durationAsString = durationToHHMMSS(durationInMs);
        this.uri = uri;
    }
}

/**
 * @param {Uri} uri
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


const PLAYLIST_CHANNEL = new BroadcastChannel('playlist_channel');
/**
 * @typedef {{'load-track': SpotifyTrack, 'play-track': SpotifyTrack, 'next-track': undefined, 'previous-track': undefined}} PlaylistEventTypes
 */

/**
 * @template {keyof PlaylistEventTypes} K
 */
class PlaylistEvent {
    /**
     * @param {K} type
     * @param {PlaylistEventTypes[K]} [payload]
     */
    constructor(type, payload) {
        this.type = type;
        this.payload = payload;
    }
}

/**
 * @template {keyof PlaylistEventTypes} K
 * @param {K} type
 * @param {PlaylistEventTypes[K]} [payload]
 */
export function dispatchPlaylistEvent(type, payload) {
    PLAYLIST_CHANNEL.postMessage(new PlaylistEvent(type, payload));
}

/**
 * @template {keyof PlaylistEventTypes} T
 * @callback PlaylistEventCallback
 * @param {PlaylistEventTypes[T]} event
 * @returns {void}
 */

/**
 * @template {keyof PlaylistEventTypes} K
 * @param {K} subscribedType
 * @param {PlaylistEventCallback<K>} callback
 */
export function subscribeToPlaylistEvent(subscribedType, callback) {
    PLAYLIST_CHANNEL.addEventListener('message', ({ data: { type, payload } }) => {
        if (type == subscribedType) {
            callback(payload);
        }
    })
};
