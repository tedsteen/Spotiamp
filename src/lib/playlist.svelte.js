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

export class TrackRow extends PlaylistRow {
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
        if (!this.trackPromise) {
            this.trackPromise = SpotifyTrack.loadFromUri(this.uri)
                .then((track) => {
                    this.track = track;
                    return track;
                })
                .catch((e) => {
                    this.loadingMessage = `Failed to load track ${this.uri.id} (${e})`;
                    throw e;
                });
        }

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
     * @type {TrackRow[]}
     */
    rows = $state([]);
    /**
     * @type {TrackRow[]}
     */
    selectedRows = $state([]);
    /**
     * The currently "active" row (keyboard focus). Used as the target for
     * scroll-into-view and as the row that gets played on Enter.
     * @type {TrackRow | undefined}
     */
    focusedRow = $state();
    /**
     * Fixed reference point for range (shift) selection.
     * @type {TrackRow | undefined}
     */
    selectionAnchor = $state();

    /**
     * @argument {string[]} uris
     */
    constructor(uris) {
        $effect(() => {
            const focusedRow = this.focusedRow;
            if (!focusedRow?.element) {
                return;
            }

            const selectedRowElement = focusedRow.element;
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
                e.preventDefault();
                if (e.altKey) {
                    this.moveSelected(1);
                } else {
                    this.selectRelative(1, e.shiftKey);
                }
            } else if (e.key == "ArrowUp") {
                e.preventDefault();
                if (e.altKey) {
                    this.moveSelected(-1);
                } else {
                    this.selectRelative(-1, e.shiftKey);
                }
            } else if (e.key == "Delete" || e.key == "Backspace") {
                e.preventDefault();
                this.removeSelected();
            } else if (e.key == "Enter") {
                e.preventDefault();
                this.playSelected();
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
                await this.addUri(SpotifyUri.fromString(uri));
            }
            // Persist after the initial load so any legacy playlist/album URIs
            // get normalised to the individual track URIs they expand into.
            this.persist();
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
        this.focusedRow = undefined;
        this.selectionAnchor = undefined;
        this.loadedRow = undefined;
        this.persist();
    }

    /**
     * Persist the current playlist as the ordered list of track URIs.
     */
    persist() {
        invoke("set_uris", { uris: this.rows.map((r) => r.uri.asString) });
    }

    /**
     * Add a single track row to the playlist (without persisting).
     * @param {SpotifyUri} uri
     */
    async addTrackRow(uri) {
        const row = new TrackRow(uri, this);
        this.rows.push(row);
        if (!this.loadedRow) {
            await row.loadTrack();
        }
    }

    /**
     * Add a URI to the playlist. Playlist/album URIs are unwrapped into their
     * individual track URIs so the playlist always consists of concrete tracks
     * (whose metadata is still lazily loaded as they enter the viewport).
     * @param {SpotifyUri} uri
     */
    async addUri(uri) {
        if (uri.type == "playlist" || uri.type == "album") {
            /** @type {string[]} */
            let trackUris;
            try {
                trackUris = await invoke("get_track_ids", { uri: uri.asString });
            } catch (e) {
                console.warn(`Could not expand ${uri.asString}`, e);
                return;
            }
            for (const trackUri of trackUris) {
                await this.addTrackRow(SpotifyUri.fromString(trackUri));
            }
        } else {
            await this.addTrackRow(uri);
        }
    }

    /**
     * @param {string[]} urls
     */
    async addUrls(urls) {
        for (const url of urls) {
            const cleanUrl = url.trim();
            try {
                if (cleanUrl.startsWith("spotify:")) {
                    await this.addUri(SpotifyUri.fromString(cleanUrl));
                } else {
                    const urlWithoutQuery = cleanUrl.split('?')[0];
                    await this.addUri(SpotifyUri.fromUrl(urlWithoutQuery));
                }
            } catch (e) {
                console.error("Failed to parse dropped URL/URI:", cleanUrl, e);
            }
        }
        this.persist();
    }

    /**
     * Update the selection for a row, mimicking native multi-select behaviour.
     *
     * @param {TrackRow} row
     * @param {{ ctrl?: boolean, shift?: boolean }} [modifiers]
     */
    select(row, { ctrl = false, shift = false } = {}) {
        const index = this.rows.indexOf(row);
        if (index === -1) {
            return;
        }

        if (shift && this.selectionAnchor) {
            const anchorIndex = this.rows.indexOf(this.selectionAnchor);
            if (anchorIndex !== -1) {
                const [start, end] =
                    anchorIndex <= index ? [anchorIndex, index] : [index, anchorIndex];
                this.selectedRows = this.rows.slice(start, end + 1);
                this.focusedRow = row;
                return;
            }
        }

        if (ctrl) {
            this.selectedRows = this.selectedRows.includes(row)
                ? this.selectedRows.filter((r) => r !== row)
                : [...this.selectedRows, row];
        } else {
            this.selectedRows = [row];
        }
        this.selectionAnchor = row;
        this.focusedRow = row;
    }

    /**
     * Move the keyboard focus by `offset`, optionally extending the selection
     * from the current anchor (shift + arrow).
     *
     * @param {number} offset
     * @param {boolean} [extend]
     */
    selectRelative(offset, extend = false) {
        const current = this.focusedRow ?? this.selectedRows[0];
        const baseIndex = current ? this.rows.indexOf(current) : -1;
        const row = this.rows[baseIndex + offset];
        if (row) {
            this.select(row, { shift: extend });
        }
    }

    /**
     * Move all selected rows as a group by one step in `offset` direction.
     *
     * @param {number} offset -1 to move up, 1 to move down
     */
    moveSelected(offset) {
        if (this.selectedRows.length === 0 || (offset !== 1 && offset !== -1)) {
            return;
        }

        const indices = this.selectedRows
            .map((r) => this.rows.indexOf(r))
            .sort((a, b) => a - b);

        // Can't move past the edges of the list.
        if (offset < 0 && indices[0] === 0) {
            return;
        }
        if (offset > 0 && indices[indices.length - 1] === this.rows.length - 1) {
            return;
        }

        // When moving down, swap from the bottom-most row first so rows don't
        // clobber each other.
        const order = offset < 0 ? indices : [...indices].reverse();
        const rows = [...this.rows];
        for (const i of order) {
            const j = i + offset;
            [rows[i], rows[j]] = [rows[j], rows[i]];
        }
        this.rows = rows;
        this.persist();
    }

    /**
     * Rebuild the rows by inserting the dragged `block` into `remaining` at
     * `insertAt` (clamped). Used for drag-to-reorder: `block` and `remaining`
     * are snapshots captured when the drag started, so only the insertion
     * point changes as the pointer moves.
     *
     * @param {TrackRow[]} block the rows being dragged
     * @param {TrackRow[]} remaining the non-dragged rows, in order
     * @param {number} insertAt insertion index within `remaining`
     */
    placeSelection(block, remaining, insertAt) {
        const clamped = Math.max(0, Math.min(remaining.length, insertAt));
        const next = [...remaining];
        next.splice(clamped, 0, ...block);
        this.rows = next;
    }

    /**
     * Remove the selected rows from the playlist, then focus a neighbouring row.
     */
    removeSelected() {
        if (this.selectedRows.length === 0) {
            return;
        }

        const removed = new Set(this.selectedRows);
        const firstIndex = Math.min(
            ...this.selectedRows.map((r) => this.rows.indexOf(r)),
        );

        if (this.loadedRow && removed.has(this.loadedRow)) {
            this.loadedRow = undefined;
        }

        this.rows = this.rows.filter((r) => !removed.has(r));

        const next = this.rows[firstIndex] ?? this.rows[firstIndex - 1];
        if (next) {
            this.select(next);
        } else {
            this.selectedRows = [];
            this.focusedRow = undefined;
            this.selectionAnchor = undefined;
        }

        this.persist();
    }

    /**
     * Play the focused row (falling back to the first selected row).
     */
    playSelected() {
        const row = this.focusedRow ?? this.selectedRows[0];
        row?.play();
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
