# emmu-gbc

Core de **Game Boy / Game Boy Color** do Emmu para o navegador. O núcleo é o [oxGBC](https://github.com/mxmgorin/oxgbc), vendorizado em `vendor/oxgbc-core` com apenas `crates/core/src` e `Cargo.toml`. No desktop, GBC roda no Mesen via `emmu-mesen-sys`.

## Dependências (todas dentro desta pasta)

```text
vendor/oxgbc-core/
├── Cargo.toml            # pacote renomeado para oxgbc-core
├── LICENSE-GPL-3.0       # GPL-3.0-only
├── UPSTREAM.txt          # fonte, commit e alterações locais
└── src/                  # APU, barramento, cartucho, CPU, emulação, PPU, utilidades
```

- Upstream: `https://github.com/mxmgorin/oxgbc`
- Commit vendorizado: `b95ff230bcfbf9ec21186cbac4b954d377a7507d`
- Licença: **GPL-3.0-only**
- Alterações locais: rename do pacote `core` → `oxgbc-core`, remoção de dev-dependencies/benchmarks/testes que exigem ROMs volumosos. O código em `src/` permanece idêntico ao upstream.

Nenhum outro crate de emulação fica fora de `crates/emmu-gbc`.

## API

Compatível com os demais cores do Emmu:

### Nativa (`GbcCore`)

- `new(rom)`, `game_title()`
- `run_frame(render_graphics)` → RGBA8888 **160 × 144**
- `take_audio_samples()` — `f32` estéreo intercalado, **44.100 Hz**, 2 canais
- `audio_sample_rate()`, `audio_channels()`, `set_volume(v)`
- `set_button(button, pressed, player)` — apenas P1; `A`, `B`, `Start`, `Select` e D-pad
- `restart()` — preserva SRAM de bateria via `dump_ram()` / `load_ram()`
- `save_state()` / `load_state(bytes)` — magic **`EMMUGBC1`**, JSON do `EmuSaveState`, inclui SRAM/MBC e RTC
- `save_ram()` / `load_save_ram()` — persistência separada da bateria

### WASM (`GbcEmulator`)

Expõe as mesmas operações com nomes camelCase via `wasm-bindgen`: `new`, `gameTitle`, `runFrame`, `audioSamples`, `audioSampleRate`, `audioChannels`, `setButton`, `setVolume`, `restart`, `saveState` e `loadState`.

## Comportamento

- O core é instanciado com `GbModel::Cgb`.
- Cartuchos DMG são coloridos por `emu.dmg_compat_palette()`; cartuchos CGB usam a paleta nativa.
- O ritmo de execução no navegador é feito por `EmuRuntime::run_frame`, não por `Emu::run_frame`.
- Save states com magic diferente são rejeitados.

## Árvore de arquivos

```text
emmu-gbc/
├── Cargo.toml        # cdylib + rlib; serde_json, wasm-bindgen, vendor path
├── LICENSE           # texto integral da GPLv3
├── README.md
├── src/
│   └── lib.rs        # GbcCore, GbcEmulator WASM, configuração e testes
└── vendor/
    └── oxgbc-core/   # GPL-3.0-only, ver LICENSE-GPL-3.0
        ├── Cargo.toml
        ├── LICENSE-GPL-3.0
        ├── UPSTREAM.txt
        └── src/
```

## Build e teste

A partir da raiz:

```sh
cargo test -p emmu-gbc
```

Com ROM real:

```sh
EMMU_GBC_TEST_ROM="/caminho/para/jogo.gbc" cargo test -p emmu-gbc -- --nocapture
```

Pacote WASM:

```sh
cd crates/emmu-gbc
wasm-pack build --target web \
  --out-dir ../../platforms/web/ui/src/wasm/gbc \
  --out-name emmu_gbc
```

Testes incluídos: `runs_frame_and_returns_rgba`, `save_state_round_trip`, `restarts_and_keeps_audio_api` e `real_rom_smoke_test` (pulado se a variável de ambiente não estiver definida).

## Desktop

No Tauri, o frontend envia `.gbc` com console `GBC`; `platforms/desktop/src/lib.rs` configura o Mesen com `AutoFavorGbc` e salva states em `.mss`. O formato `EMMUGBC1` é exclusivo do caminho WASM.

## Licenças

| Componente | Licença | Crédito |
| --- | --- | --- |
| `emmu-gbc` | GPL-3.0-only | `LICENSE` |
| oxGBC vendorizado | GPL-3.0-only | `troidem <mxmgorin@gmail.com>` — `vendor/oxgbc-core/LICENSE-GPL-3.0` |
| `serde_json`, `wasm-bindgen` | MIT OR Apache-2.0 | dependências Rust usuais |

Ao distribuir um bundle web com este crate, disponibilize o código-fonte correspondente e mantenha o texto da GPLv3, conforme a licença do core.
