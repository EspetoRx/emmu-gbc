# emmu-gbc

Core Game Boy / Game Boy Color do Emmu.

## Dependências (todas dentro desta pasta)

- `vendor/oxgbc-core`: núcleo de emulação [oxGBC](https://github.com/mxmgorin/oxgbc)
  (somente `crates/core/src`, licença GPL-3.0 em
  `vendor/oxgbc-core/LICENSE-GPL-3.0`). O commit upstream vendorizado está
  registrado em `vendor/oxgbc-core/UPSTREAM.txt`.
- `serde_json` / `wasm-bindgen`: dependências Rust normais, declaradas no
  `Cargo.toml` deste crate.

Nenhum outro crate de emulator fica fora de `crates/emmu-gbc`.

## API

Compatível com os demais cores do Emmu (`emmu-snes`, `emmu-nes`, `emmu-gba`):

- `new(rom)`, `gameTitle()`
- `runFrame(renderGraphics)` → `Uint8Array` RGBA8888 de 160x144
- `audioSamples()`, `audioSampleRate()` (44100 Hz), `audioChannels()` (2)
- `setButton(button, pressed, player)`, `setVolume(volume)`, `restart()`
- `saveState()` / `loadState(bytes)` — estado versionado `EMMUGBC1`, com SRAM
  e RTC incluídos

`GbcCore` também compila nativamente e possui `save_ram()` / `load_save_ram()`
para persistência separada da bateria.

## Desktop

No Tauri, o GBC continua rodando no Mesen (`emmu-mesen-sys`), com áudio SDL e
save states `.mss`. Este crate é usado no navegador via `wasm-pack`.

## Testes

```bash
EMMU_GBC_TEST_ROM="/path/to/game.gbc" cargo test -p emmu-gbc
```
