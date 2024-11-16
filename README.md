# Spotiamp

## Building and running
```bash
# Install all
npm i
cargo install tauri-cli

# After that run this to dev
cargo tauri dev
```

## Update version
```bash
pnpx tauri-version patch # `v0.0.2` -> `v0.0.3` - Commit message `0.0.3`
pnpx tauri-version minor # `v0.0.2` -> `v0.1.0` - Commit message `0.1.0`
pnpx tauri-version major # `v0.0.2` -> `v1.0.0` - Commit message `1.0.0`
```

## Trigger release (run the publish workflow)
```bash
git push origin tag v[version number]
```