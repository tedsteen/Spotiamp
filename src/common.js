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

export function htmlToNode(html) {
    const template = document.createElement('template');
    template.innerHTML = html;
    const nNodes = template.content.childNodes.length;
    if (nNodes !== 1) {
        throw new Error(
            `html parameter must represent a single node; got ${nNodes}. ` +
            'Note that leading or trailing spaces around an element in your ' +
            'HTML, like " <img/> ", get parsed as text nodes neighbouring ' +
            'the element; call .trim() on your input to avoid this.'
        );
    }
    return template.content.firstChild;
}

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

export const PLAYLIST_CHANNEL = new BroadcastChannel('playlist_channel');