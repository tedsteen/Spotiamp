import { preventAndStopPropagation, htmlToNode, spotifyUrlToUri, loadSpotifyUri, PLAYLIST_CHANNEL } from './common.js';
window.addEventListener("dragenter", preventAndStopPropagation);
window.addEventListener("dragover", preventAndStopPropagation);
window.addEventListener("drop", preventAndStopPropagation);
/**
 * @typedef {import('./common.js').Uri} Uri
 */

class PlaylistTrack {
    /**
     * @param {HTMLElement} parentEl 
     * @param {Uri} uri 
     */
    constructor(parentEl, uri) {
        this.uri = uri;
        this.parentEl = parentEl;
        this.trackEl = htmlToNode(`<div>Loading...</div>`);
        this.parentEl.appendChild(this.trackEl);
        //TODO: Handle failure
        loadSpotifyUri(uri).then((track) => {
            this.trackEl.innerHTML = `<div>${track.artist} - ${track.name} (${track.durationAsString})`;
            this.trackEl.addEventListener("dblclick", () => {
                PLAYLIST_CHANNEL.postMessage(track);
            })
        });
    }
}
class Playlist {
    /**
     * @param {HTMLElement} playlistEl 
     */
    constructor(playlistEl) {
        this.tracks = []
        this.playlistEl = playlistEl;
    }

    /**
     * @param {Uri} uri 
     */
    addTrack(uri) {
        this.tracks.push(new PlaylistTrack(this.playlistEl, uri))
    }
}

window.addEventListener("DOMContentLoaded", () => {
    const playlistEl = document.getElementById("playlist");
    const playlist = new Playlist(playlistEl);
    document.addEventListener("drop", (ev) => {
        for (const item of ev.dataTransfer.items) {
            if (item.kind === "string" && item.type.match(/^text\/uri-list/)) {
                item.getAsString((url) => {
                    playlist.addTrack(spotifyUrlToUri(url));
                });
            }
        }
    });

    playlistEl.addEventListener("click", (e) => {
        console.info("CLICK");
        playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/6zTO0Y58ZBd1ZMjH0EIX1X"));
    });
});
