import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * @typedef {import('./spotify.svelte').SpotifyTrack} SpotifyTrack
 */

/**
 * @typedef { {playlistWindow: {event: {Ready: null, PlayRequested: null, TrackLoaded: SpotifyTrack, EndReached: null, DragStarted: null, DragEnded: null}}, playerWindow: {event: {CloseRequested: null, UrlsDropped: string[], NextPressed: null, PreviousPressed: null, DragEnded: null }}, player: { event: { 'Paused': { uri: string, position_ms: number}, 'Playing': { uri: string, position_ms: number}, 'Stopped': {uri: string}, 'EndOfTrack': {uri: string}, 'PositionCorrection': { uri: string, position_ms: number}, 'Seeked': { uri: string, position_ms: number}} }} } WindowEventTypes
 */

/**
 * @template {keyof WindowEventTypes} T
 * @template {keyof WindowEventTypes[T]["event"]} T2
 * @param {T} key
 * @param {{[P in T2]: WindowEventTypes[T]["event"][P]}} event
 */
export async function emitWindowEvent(key, event) {
    await emit(key, event);
}

const CURRENT_WINDOW = getCurrentWindow();

/**
 * @template {keyof WindowEventTypes} T
 * @param {T} key
 * @param {(event: WindowEventTypes[T]["event"]) => void} callback
 */
export async function subscribeToWindowEvent(key, callback) {
    return await CURRENT_WINDOW.listen(key, (event) => { callback(event.payload) });
}
