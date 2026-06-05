import { invoke } from "@tauri-apps/api/core";
import { enterExitViewportObserver, REACTIVE_WINDOW_SIZE } from "./common.svelte";
import { emitWindowEvent, subscribeToWindowEvent } from "./events.svelte";
import { SpotifyTrack, SpotifyUri } from "./spotify.svelte";

class PlaylistRow {
    /**
     * @type {HTMLElement | undefined}
     */
    element = $state();

    /**
     * @param {SpotifyUri} uri
     * @param {Playlist} playlist
     */
    constructor(uri, playlist) {
        this.uri = uri;
        this.playlist = playlist;
    }

    isLoaded() {
        return false;
    }

    play() {
        // noop
    }

    getOnEnterViewport() {
        return () => { };
    }
}

class MultiTrackRow extends PlaylistRow {
    /** @type {string} */
    displayName = $state("")
    displayDuration = ""
    unavailable = false

    /**
     * @param {SpotifyUri} uri
     * @param {Playlist} playlist
     */
    constructor(uri, playlist) {
        super(uri, playlist);
        this.displayName = uri.asString;
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
            invoke("get_track_ids", { uri: self.uri.asString }).then((trackIds) => {
                self.playlist.rows.splice(self.playlist.rows.indexOf(self), 1);
                // Not sure why setTimeout is needed... Svelte bug?
                setTimeout(async () => {
                    for (const trackId of trackIds) {
                        await self.playlist.addRow(SpotifyUri.fromString(trackId));
                    }
                }, 1);
            }).catch((e) => {
                self.displayName = `Failed to load playlist ${self.uri.id} (${e})`;
            });

        };
        return eventCallback;
    }

    isSelected() {
        return this.playlist.selectedRows.includes(this);
    }
}

class TrackRow extends PlaylistRow {
    /**
     * @type {SpotifyTrack | undefined}
     */
    track = $state()
    /**
     * @type {Promise<SpotifyTrack> | undefined}
     */
    trackPromise
    loadingMessage = $state("")
    displayName = $derived(this.track ? this.track.displayName : this.loadingMessage)
    displayDuration = $derived(this.track ? this.track.displayDuration : '')
    unavailable = $derived(this.track ? this.track.unavailable : false)

    /**
     * @param {SpotifyUri} uri
     * @param {Playlist} playlist
     */
    constructor(uri, playlist) {
        super(uri, playlist);
        this.loadingMessage = `${this.uri.asString}`;
    }

    populateTrack() {
        this.trackPromise ??= SpotifyTrack.loadFromUri(this.uri)
            .then((track) => {
                this.track = track;
                return track;
            })
            .catch((e) => {
                this.loadingMessage = `Failed to load track ${this.uri.id} (${e})`;
                throw e;
            });

        return this.trackPromise;
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
            self.populateTrack().catch((/** @type {unknown} */ e) => {
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
                await emitWindowEvent("playlistWindow", { TrackLoaded: this.track });
                return this.track;
            }
        } catch (e) {
            console.warn(`Could not load track metadata for ${this.uri.id}`, e);
        }
    }

    async play() {
        await this.loadTrack();
        await emitWindowEvent("playlistWindow", { PlayRequested: null })
    }

    isLoaded() {
        return this == this.playlist.loadedRow;
    }

    isSelected() {
        return this.playlist.selectedRows.includes(this);
    }
}

export class Playlist {
    width = $derived(Math.ceil(REACTIVE_WINDOW_SIZE.width / 25));
    height = $derived(Math.ceil(REACTIVE_WINDOW_SIZE.height / 29));

    /**
     * @type {TrackRow | undefined}
     */
    loadedRow = $state();
    /**
     * @type {(TrackRow | MultiTrackRow)[]}
     */
    rows = $state([]);
    /**
     * @type {(TrackRow | MultiTrackRow)[]}
     */
    selectedRows = $state([]);

    /**
     * @argument {string[]} uris
     */
    constructor(uris) {
        $effect(() => {
            const selectedRow = this.selectedRows[0];
            if (!selectedRow?.element) {
                return;
            }

            const selectedRowElement = selectedRow.element;
            const rect = selectedRowElement.getBoundingClientRect();

            if (rect.top < 20) {
                selectedRowElement.scrollIntoView(true);
            } else if (rect.bottom > this.height * 29 - 38) {
                selectedRowElement.scrollIntoView(false);
            }
        })

        /**
        * @param {DocumentEventMap["keydown"]} e
        */
        const playlistKeyDownListener = (e) => {
            if (e.key == "ArrowDown") {
                this.selectRelative(1);
            } else if (e.key == "ArrowUp") {
                this.selectRelative(-1);
            }
        }
        document.addEventListener("keydown", playlistKeyDownListener);

        const playerWindowSubscription = subscribeToWindowEvent(
            "playerWindow",
            (event) => {
                if (event.NextPressed !== undefined) {
                    this.next(true);
                } else if (event.PreviousPressed !== undefined) {
                    this.previous(true);
                } else if (event.UrlsDropped) {
                    const urls = event.UrlsDropped;
                    this.clear().then(() => {
                        this.addUrls(urls);
                    });
                }
            },
        );

        const playerSubscription = subscribeToWindowEvent("player", (event) => {
            if (event.EndOfTrack) {
                this.next(true).then((endReached) => {
                    if (endReached) {
                        emitWindowEvent("playlistWindow", { EndReached: null });
                    }
                });
            }
        });

        (async () => {
            for (const uri of uris) {
                await this.addRow(SpotifyUri.fromString(uri));
            }
        })();

        this.dispose = () => {
            document.removeEventListener("keydown", playlistKeyDownListener);
            playerWindowSubscription.then((unlisten) => unlisten());
            playerSubscription.then((unlisten) => unlisten());
        }
    }
    async clear() {
        this.rows = [];
        this.selectedRows = [];
        this.loadedRow = undefined;
    }
    /**
     * @param {SpotifyUri} uri
     */
    async addRow(uri) {
        const row = (uri.type == "playlist" || uri.type == "album")
            ? new MultiTrackRow(uri, this)
            : new TrackRow(uri, this);
        this.rows.push(row);
        if (!this.loadedRow && row instanceof TrackRow) {
            await row.loadTrack();
        }
    }

    /**
     * @param {string[]} urls 
     */
    async addUrls(urls) {
        for (const url of urls) {
            const uri = SpotifyUri.fromUrl(url);
            await this.addRow(uri);
            invoke("add_uri", { uri: uri.asString });
        }
    }

    /**
     * @param {number} offset
     */
    selectRelative(offset) {
        const selectedRow = this.selectedRows[0];
        if (!selectedRow) {
            return;
        }

        const row = this.rows[this.rows.indexOf(selectedRow) + offset];
        if (row) {
            this.selectedRows = [row];
        }
    }

    /**
     * @param {number} offset
     * @param {boolean} skipUnavailable
     * @returns {Promise<boolean>} true if the end in that direction has been reached
     */
    async move(offset, skipUnavailable) {
        const currRowIndex = this.loadedRow ? this.rows.indexOf(this.loadedRow) : 0;
        const row = this.rows[currRowIndex + offset];
        if (!row) {
            return true;
        }

        if (row instanceof TrackRow) {
            const track = await row.loadTrack();
            if (track && skipUnavailable && track.unavailable) {
                return await this.move(offset, skipUnavailable);
            }
        }

        return false;
    }

    /**
     * @param {boolean} skipUnavailable
     * @returns {Promise<boolean>} true if end has been reached
     */
    async next(skipUnavailable = false) {
        return await this.move(1, skipUnavailable);
    }

    /**
     * @param {boolean} skipUnavailable
     * @returns {Promise<boolean>} true if top has been reached
     */
    async previous(skipUnavailable = false) {
        return await this.move(-1, skipUnavailable);
    }

}
