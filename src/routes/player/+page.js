import { invoke } from '@tauri-apps/api/core';

/** @type {import('./$types').PageLoad} */
export async function load({ }) {
	/**
	 * @type {number}
	 */
	return {
		initialVolume: await invoke("get_volume")
	};
}