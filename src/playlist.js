import { handleError, preventAndStopPropagation, spotifyUrlToUri, getTrack, dispatchPlaylistEvent, subscribeToPlaylistEvent } from './common.js';
window.addEventListener("dragenter", preventAndStopPropagation);
window.addEventListener("dragover", preventAndStopPropagation);
window.addEventListener("drop", preventAndStopPropagation);
/**
 * @typedef {import('./common.js').Uri} Uri
 */

const BLANK_CANVAS = document.createElement('canvas');
document.body?.appendChild(BLANK_CANVAS);
class PlaylistTrack {
    /**
     * @param {Playlist} playlist
     * @param {Uri} uri 
     */
    constructor(playlist, uri) {
        this.playlist = playlist;
        this.uri = uri;
        this.track = getTrack(uri);

        const template = document.querySelector("#template-playlist-track-row");
        const clone = template?.content.cloneNode(true);
        /**
         * @type HTMLElement
         */
        const trackEl = clone.querySelector(".playlist-track");
        trackEl.track = this;
        this.el = trackEl;
        const trackNameEl = clone.querySelector(".playlist-track-name");
        const trackDurationEl = clone.querySelector(".playlist-track-duration");
        playlist.el.appendChild(clone);

        this.track.then((track) => {
            trackNameEl.textContent = `${track.artist} - ${track.name}`;
            trackDurationEl.textContent = track.durationAsString;
        }).catch((e) => {
            handleError(e);
            trackEl.remove();
        });

        trackEl.addEventListener("mousedown", () => {
            playlist.el.querySelectorAll(".playlist-track").forEach((trackEl) => {
                trackEl.classList.remove("selected");
            })
            trackEl.classList.add("selected");
        });

        trackEl.addEventListener("dragstart", (ev) => {
            ev.dataTransfer?.setDragImage(BLANK_CANVAS, 0, 0);
            PlaylistTrack.row = trackEl;
        });

        trackEl.addEventListener("dragover", () => {
            const children = [...playlist.el.children];
            if (children.indexOf(trackEl) > children.indexOf(PlaylistTrack.row)) {
                trackEl.after(PlaylistTrack.row);
            } else {
                trackEl.before(PlaylistTrack.row);
            }
        });

        trackEl.addEventListener("dblclick", () => {
            this.load();
        })
    }
    /**
     * @param {SpotifyTrack} track 
     */
    load() {
        let track = this.track;
        track.then((track) => {
            dispatchPlaylistEvent('load-track', track);
        });

        this.playlist.loadedTrack = this;

        this.playlist.el.querySelectorAll(".playlist-track").forEach((trackEl) => {
            trackEl.classList.remove("loaded");
        })
        this.el.classList.add("loaded");
    }
}
class Playlist {
    /**
     * @param {HTMLElement} playlistEl 
     */
    constructor(playlistEl) {
        this.tracks = []
        this.el = playlistEl;
        /**
         * @type PlaylistTrack | undefined
         */
        this.loadedTrack = undefined;
    }

    /**
     * @param {Uri} uri 
     */
    addTrack(uri) {
        this.tracks.push(new PlaylistTrack(this, uri))
    }

    next() {
        if (this.loadedTrack?.el) {
            const nextTrackEl = this.loadedTrack?.el.nextElementSibling;
            if (nextTrackEl) {
                nextTrackEl.track.load();
            }
        }
    }
    previous() {
        if (this.loadedTrack?.el) {
            const previousTrackEl = this.loadedTrack?.el.previousElementSibling;
            if (previousTrackEl) {
                previousTrackEl.track.load();
            }
        }
    }
}

window.addEventListener("DOMContentLoaded", () => {
    const playlist = new Playlist(document.getElementById("playlist"));
    playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/6zTO0Y58ZBd1ZMjH0EIX1X"));
    playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/72oaFIAqlK7N7a8cyHZZ3i"));
    playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/6qnoOnDK3embwtU89Fz5XN"));
    playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/1D9sYkaLD1cOBeyLRzkXzI"));
    playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/3ZuT0Evo8chdVM6rPXXqgd"));
    playlist.addTrack(spotifyUrlToUri("https://open.spotify.com/track/0urZ9y68sA2mlWKbfxn8KU"));

    document.addEventListener("drop", (ev) => {
        for (const item of ev.dataTransfer.items) {
            if (item.kind === "string" && item.type.match(/^text\/uri-list/)) {
                item.getAsString((url) => {
                    playlist.addTrack(spotifyUrlToUri(url));
                });
            }
        }
    });

    subscribeToPlaylistEvent('next-track', () => {
        playlist.next();
    });

    subscribeToPlaylistEvent('previous-track', () => {
        playlist.previous();
    });
});
