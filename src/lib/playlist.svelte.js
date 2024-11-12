// TODO: only import the type somehow
import { invoke } from "@tauri-apps/api/core";
import { handleError, loadTrack, SpotifyTrack, SpotifyUri, subscribeToWindowEvent } from "./common";

class PlaylistRow {
    /**
     * @type {SpotifyUri}
     */
    uri
    /**
     * @type {SpotifyTrack | undefined}
     */
    track = $state()
    loadingMessage = $state()
    displayName = $derived(this.track ? this.track.displayName : this.loadingMessage)
    displayDuration = $derived(this.track ? this.track.displayDuration : '')

    /**
     * @param {SpotifyUri} uri
     * @param {Playlist} playlist
     */
    constructor(uri, playlist) {
        this.playlist = playlist;
        this.uri = uri;
        this.loadingMessage = `${this.uri}`;
    }

    /**
     * @param {HTMLElement} e 
     */
    actionWhenInViewport(e) {
        const observer = new IntersectionObserver(entries => {
            if (entries[0].isIntersecting) {
                if (this.uri.type == "track") {
                    loadTrack(this.uri).then((track) => {
                        console.info("Populate track", track);
                        this.track = track;
                    })
                } else if (this.uri.type == "playlist") {
                    invoke("get_playlist_track_ids", { uri: this.uri.toString() }).then((trackIds) => {
                        // Remove the loading-playlist-row
                        this.playlist.rows.splice(this.playlist.rows.indexOf(this));
                        for (var trackId of trackIds) {
                            this.playlist.addTrack(SpotifyUri.fromString(trackId))
                        }
                    })
                }
                observer.disconnect();
            }
        });

        observer.observe(e);

        return {
            destroy() {
                observer.disconnect();
            }
        }
    }

    async load() {
        if (this.track) {
            this.playlist.loadedRow = this;
            await invoke("load_track", { uri: this.track.uri.toString() }).catch(handleError);
        }
    }

    play() {
        this.load().then(() => {
            invoke("play", {}).catch(handleError);
        });
    }

    isLoaded() {
        return this == this.playlist.loadedRow;
    }

    isSelected() {
        return this.playlist.selectedRows.indexOf(this) != -1;
    }
}

export class Playlist {
    /**
     * @type {PlaylistRow | undefined}
     */
    loadedRow = $state();
    /**
     * @type {PlaylistRow[]}
     */
    rows = $state([]);
    /**
     * @type {PlaylistRow[]}
     */
    selectedRows = $state([]);
    constructor() {
        /**
        * @param {DocumentEventMap["keydown"]} e
        */
        const playlistKeyDownListener = (e) => {
            const selectedRow = this.selectedRows[0];
            if (selectedRow) {
                let nextRow;
                if (e.key == "ArrowDown") {
                    const currRowIndex = this.rows.indexOf(selectedRow);
                    nextRow = this.rows[currRowIndex + 1];
                } else if (e.key == "ArrowUp") {
                    const currRowIndex = this.rows.indexOf(selectedRow);
                    nextRow = this.rows[currRowIndex - 1];
                }
                if (nextRow) {
                    this.selectedRows = [nextRow];
                }
            }
        }
        document.addEventListener("keydown", playlistKeyDownListener);

        const playerWindowSubscription = subscribeToWindowEvent(
            "playerWindow",
            (event) => {
                if (event.NextPressed !== undefined) {
                    this.playNext();
                } else if (event.PreviousPressed !== undefined) {
                    this.playPrevious();
                }
            },
        );

        const playerSubscription = subscribeToWindowEvent("player", (event) => {
            if (event.EndOfTrack) {
                if (!this.playNext()) {
                    invoke("stop").catch(handleError);
                }
            }
        });

        this.dispose = () => {
            document.removeEventListener("keydown", playlistKeyDownListener);
            playerWindowSubscription.then((unlisten) => unlisten());
            playerSubscription.then((unlisten) => unlisten());
        }
    }

    /**
     * @param {SpotifyUri} uri
     */
    addTrack(uri) {
        const row = new PlaylistRow(uri, this);
        this.rows.push(row);
        if (!this.loadedRow) {
            row.load();
        }
    }

    playNext() {
        const currRowIndex = this.loadedRow ? this.rows.indexOf(this.loadedRow) : 0;
        const nextRow = this.rows[currRowIndex + 1];
        if (nextRow) {
            nextRow.load();
            return true;
        } else {
            return false;
        }
    }

    playPrevious() {
        const currRowIndex = this.loadedRow ? this.rows.indexOf(this.loadedRow) : 0;
        const previousRow = this.rows[currRowIndex - 1];
        previousRow?.load();
    }

}