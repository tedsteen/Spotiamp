import { invoke } from '@tauri-apps/api/core';

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
    const durationInSeconds = Math.floor(durationInMs / 1000);
    const minutes = Math.floor(durationInSeconds / 60);
    const seconds = durationInSeconds - minutes * 60;
    return new MMSS(minutes, seconds);
}

/**
 * @param {number} durationInMs
 * @returns {string}
 */
export function durationToString(durationInMs) {
    const { m, s } = durationToMMSS(durationInMs);
    return `${m.toString().padStart(1, '0')}:${s.toString().padStart(2, '0')}`;
}

/**
 * Winamp's playlist running-time format: "M:SS", or "H:MM:SS" from an hour.
 * @param {number} durationInMs
 * @returns {string}
 */
export function durationToRunningTime(durationInMs) {
    const totalSeconds = Math.floor(durationInMs / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = (totalSeconds % 60).toString().padStart(2, "0");
    return hours > 0
        ? `${hours}:${minutes.toString().padStart(2, "0")}:${seconds}`
        : `${minutes}:${seconds}`;
}

export class SpotifyUri {
    /**
     * @param {"track" | "playlist" | "album"} type
     * @param {string} id
     */
    constructor(type, id) {
        this.type = type;
        this.id = id;
        this.asString = `spotify:${this.type}:${this.id}`;
    }

    /**
     * @param {string} uriAsString
     */
    static fromString(uriAsString) {
        const matches = spotifyUriRe.exec(uriAsString);
        if (matches?.length == 3) {
            const type = matches[1], id = matches[2];
            if (type == "track" || type == "playlist" || type == "album") {
                return new SpotifyUri(type, id);
            }
            throw `'${uriAsString}' is not a valid spotify URI. Only track, playlist and album types are allowed`;
        }

        throw `${uriAsString} does not match a spotify URI`;
    }

    /**
     * @param {string} url
     * @returns {SpotifyUri}
     */
    static fromUrl(url) {
        const matches = spotifyUrlRe.exec(url);
        if (matches?.length == 3) {
            const type = matches[1], id = matches[2];
            return SpotifyUri.fromString(`spotify:${type}:${id}`);
        }

        throw `${url} does not match a spotify URL`;
    }

    /**
     * Parse either a `spotify:` URI or an open.spotify.com URL.
     * @param {string} text
     * @returns {SpotifyUri}
     */
    static parse(text) {
        const trimmed = text.trim();
        return trimmed.startsWith("spotify:")
            ? SpotifyUri.fromString(trimmed)
            : SpotifyUri.fromUrl(trimmed);
    }
}

const spotifyUriRe = /spotify:(.*):(.{22})/;
// Also matches localised links (open.spotify.com/intl-sv/track/...) and ignores
// any trailing query string (?si=...).
const spotifyUrlRe = /https:\/\/open\.spotify\.com\/(?:intl-[\w-]+\/)?(\w+)\/([A-Za-z0-9]{22})/;

/**
 * @typedef {{uri: string, name: string, subtitle: string, info: string, playable: boolean}} BrowseItem
 * @typedef {"tracks" | "albums" | "playlists"} SearchKind
 */

/**
 * Search Spotify (done in the backend through the librespot session).
 * @param {string} query
 * @param {SearchKind} kind
 * @param {number} [offset]
 * @returns {Promise<{items: BrowseItem[], next_offset: number | null}>}
 */
export function searchSpotify(query, kind, offset = 0) {
    return invoke("search_spotify", { query, kind, offset });
}

/**
 * The playlists in the user's Spotify library.
 * @returns {Promise<BrowseItem[]>}
 */
export function getMyPlaylists() {
    return invoke("get_my_playlists");
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
        this.durationInMs = durationInMs;
        this.displayDuration = durationToString(durationInMs);
        this.displayName = `${this.artist} - ${this.name}`;
        this.uri = uri;
        this.unavailable = unavailable;
    }

    /**
     * @param {SpotifyUri} uri
     * @returns {Promise<SpotifyTrack>}
     */
    static async loadFromUri(uri) {
        /** @type {{artist: string, name: string, duration: number, uri: string, unavailable: boolean}} */
        const trackData = await invoke("get_track_metadata", { uri: uri.asString });
        return new SpotifyTrack(trackData.artist, trackData.name, trackData.duration, uri, trackData.unavailable);
    }
}
