const { message } = window.__TAURI__.dialog;
const { invoke } = window.__TAURI__.core;
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
export function preventAndStopPropagation(ev) {
    ev.preventDefault();
    ev.stopPropagation();
}

const zoom = window.innerWidth / 275.0;
document.querySelector(':root').style.setProperty('--zoom', zoom);

const spotifyUrlRe = /https:\/\/open.spotify.com\/(.*)\/(.{22})/;

/**
 * @param {string} url
 * @returns {string}
 */
export function spotifyUrlToUri(url) {
    const matches = spotifyUrlRe.exec(url);
    return `spotify:${matches[1]}:${matches[2]}`;
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
    * @returns {Promise<SpotifyTrack>}
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
export async function getTrack(uri) {
    const trackData = await invoke("get_track", { uri });
    return new SpotifyTrack(trackData.artist, trackData.name, trackData.duration, uri);
}

const PLAYLIST_CHANNEL = new BroadcastChannel('playlist_channel');
/**
 * @typedef {{'load-track': SpotifyTrack, 'next-track': undefined, 'previous-track': nothing}} PlaylistEventTypes
 */

class PlaylistEvent {
    /**
     * @template {keyof PlaylistEventTypes} K
     * @param {K} type
     * @param {PlaylistEventTypes[K]} payload
     */
    constructor(type, payload) {
        this.type = type;
        this.payload = payload;
    }
}

/**
 * @template {keyof PlaylistEventTypes} K
 * @param {K} type
 * @param {PlaylistEventTypes[K]} payload
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
