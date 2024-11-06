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

/**
 * @typedef {(string)} Uri
 */

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
        this.durationInMs = durationInMs
        this.durationAsString = durationToString(durationInMs);
        this.uri = uri;
    }
}