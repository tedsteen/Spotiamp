# Spotiamp

A Spotify client with a Winamp Classic (2.95) look.
It is work in progress, mostly a fun project for me personally to try out Svelte 5 and Tauri 2.
If you like it and miss your favourite Winamp Classic feature add an issue about it or even better create a PR.

Check [releases](https://github.com/tedsteen/Spotiamp/releases/) for MacOS and Windows binaries.
<p align="center">
  <img src="player.png" alt="Player"/>
  <br />
  <img src="playlist.png" alt="Playlist"/>
</p>

## Building and running
Requires Node 22.17+ and a stable Rust toolchain.
```bash
# Install all
npm i

# After that run this to dev
npm run tauri dev
```

## Update version
```bash
npx tauri-version patch # `v0.0.2` -> `v0.0.3` - Commit message `0.0.3`
npx tauri-version minor # `v0.0.2` -> `v0.1.0` - Commit message `0.1.0`
npx tauri-version major # `v0.0.2` -> `v1.0.0` - Commit message `1.0.0`
```

## Trigger release (run the publish workflow)
```bash
git push origin tag v[version number]
```