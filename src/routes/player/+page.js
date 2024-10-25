import { invoke } from '@tauri-apps/api/core';

/** @type {import('./$types').PageLoad} */
export async function load({ }) {
	/**
	 * @type {number}
	 */
	const initialVolume = await invoke("get_volume");
	return {
		initialVolume
	};
}