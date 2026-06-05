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
}

const spotifyUriRe = /spotify:(.*):(.{22})/;
const spotifyUrlRe = /https:\/\/open.spotify.com\/(.*)\/(.{22})/;

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
