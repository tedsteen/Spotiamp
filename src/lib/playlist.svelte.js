// TODO: only import the type somehow
import { invoke } from "@tauri-apps/api/core";
import { emitWindowEvent, enterExitViewportObserver, handleError, SpotifyTrack, SpotifyUri, subscribeToWindowEvent } from "./common";
import memoize from "lodash.memoize";

class PlaylistRow {
    displayDuration = ""
    /**
     * @type {string}
     */
    displayName = $state("")
    unavailable = false

    /**
     * @param {SpotifyUri} uri
     * @param {Playlist} playlist
     */
    constructor(uri, playlist) {
        this.uri = uri;
        this.playlist = playlist;
        this.displayName = uri.asString;
    }

    isLoaded() {
        return false;
    }

    isSelected() {
        return this.playlist.selectedRows.indexOf(this) != -1;
    }

    play() {
        //noop
    }

    getOnEnterViewport() {
        // NOTE: `this` is overridden with the HTMLElement when attaching event listeners to elements.
        //       We capture `this` as `self` before returning the actual event callback so that we can access `this` in the callback.
        const self = this;
        /**
         * @this HTMLElement
         */
        function eventCallback() {
            enterExitViewportObserver.unobserve(this);
            invoke("get_playlist_track_ids", { uri: self.uri.asString }).then((trackIds) => {
                self.playlist.rows.splice(self.playlist.rows.indexOf(self), 1);
                // Not sure why setTimeout is needed... Svelte bug?
                setTimeout(() => {
                    for (var trackId of trackIds) {
                        self.playlist.addRow(SpotifyUri.fromString(trackId))
                    }
                }, 1);
            }).catch((e) => {
                self.displayName = `Failed to load playlist ${self.uri.id} (${e})`;
            });

        };
        return eventCallback;
    }
}

class TrackRow {
    /**
     * @type {SpotifyTrack | undefined}
     */
    track = $state()
    loadingMessage = $state("")
    displayName = $derived(this.track ? this.track.displayName : this.loadingMessage)
    displayDuration = $derived(this.track ? this.track.displayDuration : '')
    unavailable = $derived(this.track ? this.track.unavailable : false)

    /**
     * @param {SpotifyUri} uri
     * @param {Playlist} playlist
     */
    constructor(uri, playlist) {
        this.playlist = playlist;
        this.uri = uri;
        this.loadingMessage = `${this.uri.asString}`;
        this.populateTrack = memoize(() => {
            /**
             * @type {Promise<SpotifyTrack>}
             */
            const promise = new Promise((resolve, reject) => {
                SpotifyTrack.loadFromUri(this.uri).then((track) => {
                    this.track = track;
                    resolve(track);
                }).catch((e) => {
                    this.loadingMessage = `Failed to load track ${this.uri.id} (${e})`;
                    reject(e);
                });
            });
            return promise;
        });
    }

    getOnEnterViewport() {
        // NOTE: `this` is overridden with the HTMLElement when attaching event listeners to elements.
        //       We capture `this` as `self` before returning the actual event callback so that we can access `this` in the callback.
        const self = this;
        /**
         * @this HTMLElement
         */
        function eventCallback() {
            enterExitViewportObserver.unobserve(this);
            self.populateTrack().catch((e) => {
                console.warn(`Could not load metadata for ${self.uri.id}`, e);
            });
        };
        return eventCallback;
    }

    async loadTrack() {
        try {
            await this.populateTrack();
            if (this.track) {
                this.playlist.loadedRow = this;
                await emitWindowEvent("playlistWindow", { LoadTrack: { track: this.track } });
            }
        } catch (e) {
            console.warn(`Could not load track metadata for ${this.uri.id}`, e);
        }
    }

    async play() {
        if (!this.track?.unavailable) {
            await this.loadTrack();
            await emitWindowEvent("playlistWindow", { PlayRequsted: null })
        }
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
     * @type {TrackRow | undefined}
     */
    loadedRow = $state();
    /**
     * @type {(TrackRow | PlaylistRow)[]}
     */
    rows = $state([]);
    /**
     * @type {(TrackRow | PlaylistRow)[]}
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
    async addRow(uri) {
        const row = uri.type == "playlist" ? new PlaylistRow(uri, this) : new TrackRow(uri, this);
        this.rows.push(row);
        if (!this.loadedRow && row instanceof TrackRow) {
            await row.loadTrack();
        }
    }

    playNext() {
        const currRowIndex = this.loadedRow ? this.rows.indexOf(this.loadedRow) : 0;
        const nextRow = this.rows[currRowIndex + 1];

        if (nextRow) {
            if (nextRow instanceof TrackRow) {
                nextRow.loadTrack();
            }
            return true;
        } else {
            return false;
        }
    }

    playPrevious() {
        const currRowIndex = this.loadedRow ? this.rows.indexOf(this.loadedRow) : 0;
        const previousRow = this.rows[currRowIndex - 1];
        if (previousRow instanceof TrackRow) {
            previousRow.loadTrack();
        }
    }

}