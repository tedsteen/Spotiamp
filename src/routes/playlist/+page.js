import { dispatchWindowChannelEvent, subscribeToWindowChannelEvent } from '$lib/windowChannel';

/** @type {import('./$types').PageLoad} */
export async function load({ }) {
    dispatchWindowChannelEvent('ping-player');
    await new Promise((resolve) => {
        subscribeToWindowChannelEvent("player-ready", resolve);
    });

    return {};
}