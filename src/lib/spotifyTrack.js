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
        this.durationAsString = durationToHHMMSS(durationInMs);
        this.uri = uri;
    }
}