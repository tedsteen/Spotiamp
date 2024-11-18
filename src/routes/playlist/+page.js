import { invoke } from '@tauri-apps/api/core';

/** @type {import('./$types').PageLoad} */
export async function load({ }) {
	/**
	 * @type {import('$lib/common.svelte').PlaylistSettings}
	 */
	const playlistSettings = await invoke("get_playlist_settings");
	return playlistSettings;
}