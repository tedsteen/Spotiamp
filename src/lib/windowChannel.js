import { SpotifyTrack } from "$lib/spotifyTrack";

const WINDOW_CHANNEL = new BroadcastChannel('window_channel');
/**
 * @typedef {{'load-track': SpotifyTrack, 'play-track': SpotifyTrack, 'next-track': undefined, 'previous-track': undefined}} WindowChannelEventTypes
 */

/**
 * @template {keyof WindowChannelEventTypes} K
 */
class WindowChannelEvent {
    /**
     * @param {K} type
     * @param {WindowChannelEventTypes[K]} [payload]
     */
    constructor(type, payload) {
        this.type = type;
        this.payload = payload;
    }
}

/**
 * @template {keyof WindowChannelEventTypes} K
 * @param {K} type
 * @param {WindowChannelEventTypes[K]} [payload]
 */
export function dispatchWindowChannelEvent(type, payload) {
    WINDOW_CHANNEL.postMessage(new WindowChannelEvent(type, payload));
}

/**
 * @template {keyof WindowChannelEventTypes} T
 * @callback WindowChannelEventCallback
 * @param {WindowChannelEventTypes[T]} event
 * @returns {void}
 */

/**
 * @template {keyof WindowChannelEventTypes} K
 * @param {K} subscribedType
 * @param {WindowChannelEventCallback<K>} callback
 */
export function subscribeToWindowChannelEvent(subscribedType, callback) {
    WINDOW_CHANNEL.addEventListener('message', ({ data: { type, payload } }) => {
        if (type == subscribedType) {
            callback(payload);
        }
    })
};