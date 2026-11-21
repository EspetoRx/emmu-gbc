# oxGBC (cópia vendorizada para emmu-gbc)

Código do núcleo [oxGBC](https://github.com/mxmgorin/oxgbc) incorporado ao crate `emmu-gbc` para emulação de **Game Boy / Game Boy Color** no navegador.

## Procedência

Registro completo em [`UPSTREAM.txt`](UPSTREAM.txt):

| Campo | Valor |
| --- | --- |
| Fonte | `https://github.com/mxmgorin/oxgbc` |
| Caminho original | `crates/core` (apenas `src/` e `Cargo.toml`) |
| Commit | `b95ff230bcfbf9ec21186cbac4b954d377a7507d` |
| Licença | **GPL-3.0-only** |
| Autor upstream | `troidem <mxmgorin@gmail.com>` |

## Modificações locais

- O nome do pacote foi alterado de `core` para `oxgbc-core`.
- Dev-dependencies, benchmarks e testes que exigem ROMs volumosos foram removidos.
- O código em `src/` permanece idêntico ao upstream.

## Árvore de arquivos

```text
oxgbc-core/
├── Cargo.toml          # pacote oxgbc-core 0.1.0, edition 2021
├── LICENSE-GPL-3.0     # texto integral da GPLv3
├── UPSTREAM.txt        # proveniência e alterações
└── src/
    ├── apu.rs          # APU e geração de áudio
    ├── aux.rs          # utilidades
    ├── bus.rs          # barramento e memória
    ├── cart.rs         # cartuchos/MBC/RAM/RTC
    ├── cpu.rs          # CPU LR35902
    ├── emu.rs          # orquestração, runtime e save state
    ├── lib.rs          # API pública do core
    └── ppu.rs          # PPU e framebuffer
```

## Uso no Emmu

O crate `emmu-gbc` instancia este core com `GbModel::Cgb`. Cartuchos DMG recebem a paleta de compatibilidade do próprio oxGBC; cartuchos CGB usam a paleta nativa. `GbcCore` envolve `Emu`/`EmuRuntime`, aplica volume, converte áudio para `f32` estéreo e serializa o estado com o magic `EMMUGBC1`. No desktop, GBC é executado pelo Mesen.

## Licença

GPL-3.0-only. O texto integral está em [`LICENSE-GPL-3.0`](LICENSE-GPL-3.0). Preserve os avisos e disponibilize o código-fonte ao distribuir o bundle web.
