import { invoke } from '@tauri-apps/api/core';

/** @type {import('./$types').PageLoad} */
export async function load({ }) {
	/**
	 * @type {import('$lib/common.svelte').PlayerSettings}
	 */
	const playerSettings = await invoke("get_player_settings");
	return playerSettings;
}