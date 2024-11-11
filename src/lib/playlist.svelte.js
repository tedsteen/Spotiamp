// TODO: only import the type somehow
import { SpotifyTrack } from "$lib/spotifyTrack";
import { invoke } from "@tauri-apps/api/core";
import { handleError, subscribeToWindowEvent } from "./common";

class PlaylistRow {
    /**
     * @param {SpotifyTrack} track
     * @param {Playlist} playlist
     */
    constructor(track, playlist) {
        this.track = track;
        this.playlist = playlist;
    }

    async load() {
        this.playlist.loadedRow = this;
        await invoke("load_track", { uri: this.track.uri }).catch(handleError);
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
     * @param {SpotifyTrack} track
     */
    addTrack(track) {
        const row = new PlaylistRow(track, this);
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