//! Core Game Boy / Game Boy Color do Emmu.
//!
//! O núcleo de emulação é o oxGBC, vendorizado em `vendor/oxgbc-core` (GPL-3.0).
//! A camada deste crate expõe a mesma API dos demais cores (`emmu-snes`,
//! `emmu-nes`, `emmu-gba`) consumed pelo worker de WebAssembly:
//!
//! - `new(rom)` / `gameTitle()`
//! - `runFrame(renderGraphics)` → RGBA8888 (160x144)
//! - `audioSamples()` / `audioSampleRate()` / `audioChannels()`
//! - `setButton(button, pressed, player)`
//! - `saveState()` / `loadState(bytes)` / `restart()`
//!
//! No desktop o Game Boy Color roda no Mesen via `emmu-mesen-sys`; este crate
//! é usado no navegador e também pode ser usado como biblioteca Rust nativa.

use oxgbc_core::apu::apu::ApuConfig;
use oxgbc_core::apu::{Apu, SAMPLING_FREQUENCY};
use oxgbc_core::auxiliary::io::Io;
use oxgbc_core::auxiliary::joypad::JoypadButton;
use oxgbc_core::bus::Bus;
use oxgbc_core::cart::Cart;
use oxgbc_core::emu::config::{EmuConfig, GbModel};
use oxgbc_core::emu::runtime::{EmuRuntime, EmuSaveState};
use oxgbc_core::emu::{Emu, EmuAudioCallback};
use oxgbc_core::ppu::lcd::Lcd;
use oxgbc_core::ppu::tile::PixelColor;
use oxgbc_core::ppu::{LCD_X_RES, LCD_Y_RES, Ppu};

/// Largura da tela do Game Boy (pixel a pixel, sem upscale).
pub const FRAME_WIDTH: usize = LCD_X_RES as usize;
/// Altura da tela do Game Boy.
pub const FRAME_HEIGHT: usize = LCD_Y_RES as usize;
/// Taxa de amostragem estereofônica do APU.
pub const AUDIO_SAMPLE_RATE: u32 = SAMPLING_FREQUENCY;
/// Número de canais de áudio expostos ao frontend (estéreo intercalado).
pub const AUDIO_CHANNELS: u32 = 2;

/// Prefixo dos save states do Emmu, para rejeitar dados de outros cores.
const STATE_MAGIC: &[u8; 8] = b"EMMUGBC1";

/// Paleta verde clássica do DMG, usada quando o cartucho não fornece uma
/// colorização CGB própria (jogos monocromáticos em hardware GB).
fn default_dmg_colors() -> [PixelColor; 4] {
    [
        PixelColor::from_hex_rgba("E0F8D0FF"),
        PixelColor::from_hex_rgba("88C070FF"),
        PixelColor::from_hex_rgba("346856FF"),
        PixelColor::from_hex_rgba("081820FF"),
    ]
}

/// Coleta as amostras do APU durante um frame. O volume é aplicado aqui para
/// que `setVolume` funcione também no navegador (o worker não usa Web Audio
/// diretamente, mas a API fica compatível com a do GBA).
#[derive(Default)]
struct AudioSink {
    samples: Vec<f32>,
    volume: f32,
}

impl EmuAudioCallback for AudioSink {
    fn update(&mut self, output: &[f32], _runtime: &EmuRuntime) {
        if self.volume <= 0.0 {
            return;
        }
        self.samples
            .extend(output.iter().map(|sample| sample * self.volume));
    }
}

/// Core Rust puro. Usado diretamente nos testes e embrulhado pelo
/// `GbcEmulator` quando compilado para `wasm32`.
pub struct GbcCore {
    emu: Emu,
    audio: AudioSink,
    rom: Box<[u8]>,
    frame: Vec<u8>,
    game_title: String,
}

impl GbcCore {
    pub fn new(rom_data: &[u8]) -> Result<Self, String> {
        if rom_data.len() < 0x150 {
            return Err("ROM Game Boy inválida: cabeçalho muito curto.".to_owned());
        }

        let config = EmuConfig {
            // Um GBC executa tanto carts CGB quanto carts DMG (modo
            // compatibilidade). O oxGBC usa isto para configurar PPU/APU.
            model: Some(GbModel::Cgb),
            ..EmuConfig::default()
        };
        let emu = Self::build_emu(config.clone());

        let rom = rom_data.to_vec().into_boxed_slice();
        let cart = Cart::new(rom.clone())?;
        let game_title = cart.data.get_title();

        let mut core = Self {
            emu,
            audio: AudioSink {
                volume: 1.0,
                ..AudioSink::default()
            },
            rom,
            frame: Vec::with_capacity(FRAME_WIDTH * FRAME_HEIGHT * 4),
            game_title,
        };
        core.load_cart(None);
        Ok(core)
    }

    fn build_emu(config: EmuConfig) -> Emu {
        let lcd = Lcd::new(default_dmg_colors(), GbModel::Cgb);
        let ppu = Ppu::new(lcd);
        let apu = Apu::new(ApuConfig::default());
        let bus = Bus::new(Cart::empty(), Io::new(ppu, apu), config.model);
        Emu::new(config, EmuRuntime::new(bus)).expect("falha ao construir o emulador GBC")
    }

    /// (Re)carrega o cartucho. `save_ram`, quando presente, restaura o SRAM de
    /// bateria (usado pelo restart para não perder o progresso do jogador).
    fn load_cart(&mut self, save_ram: Option<Box<[u8]>>) {
        let mut cart = Cart::new(self.rom.clone()).expect("ROM já validada em new()");
        if let Some(ram) = save_ram {
            cart.load_ram(ram);
        }

        self.emu.load_cart(cart);

        // DMG colorizado pelo boot ROM do CGB, como no oxGBC desktop.
        let compat = self.emu.dmg_compat_palette();
        let lcd = &mut self.emu.runtime.cpu.clock.bus.io.ppu.lcd;
        if let Some(compat) = compat {
            lcd.set_model(GbModel::Dmg);
            compat.apply(&mut lcd.dmg_palette);
        } else {
            lcd.dmg_palette.set_colors(default_dmg_colors());
        }

        self.audio.samples.clear();
        self.frame.clear();
    }

    pub fn game_title(&self) -> &str {
        &self.game_title
    }

    /// Avança exatamente um frame. Com `render_graphics == false` ainda
    /// avança a emulação e o áudio, mas não converte o framebuffer.
    pub fn run_frame(&mut self, render_graphics: bool) -> Result<Vec<u8>, String> {
        // `Emu::run_frame` aplica pacing próprio (sleep/spin). No navegador o
        // rAF já faz o pacing, então usamos o runtime diretamente.
        self.emu.runtime.run_frame(&mut self.audio);

        if !render_graphics {
            return Ok(Vec::new());
        }

        let rgb = self.emu.get_framebuffer().rgb888();
        self.frame.clear();
        self.frame.reserve(rgb.len() / 3 * 4);
        for pixel in rgb.chunks_exact(3) {
            self.frame.extend_from_slice(pixel);
            self.frame.push(0xFF);
        }
        Ok(self.frame.clone())
    }

    /// Amostras estereofônicas intercaladas (L, R, L, R, ...) produzidas
    /// desde a última chamada.
    pub fn take_audio_samples(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.audio.samples)
    }

    pub fn audio_sample_rate(&self) -> u32 {
        AUDIO_SAMPLE_RATE
    }

    pub fn audio_channels(&self) -> u32 {
        AUDIO_CHANNELS
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.audio.volume = volume.clamp(0.0, 1.0) as f32;
    }

    /// Atualiza um botão. Game Boy tem apenas 8 botões: X/Y/L/R são
    /// ignorados (o frontend os envia para todos os cores).
    pub fn set_button(&mut self, button: &str, pressed: bool, player: u8) -> Result<(), String> {
        if player != 1 {
            return Ok(());
        }
        let button = match button {
            "A" => JoypadButton::A,
            "B" => JoypadButton::B,
            "Start" => JoypadButton::Start,
            "Select" => JoypadButton::Select,
            "Up" => JoypadButton::Up,
            "Down" => JoypadButton::Down,
            "Left" => JoypadButton::Left,
            "Right" => JoypadButton::Right,
            "X" | "Y" | "L" | "R" => return Ok(()),
            _ => return Err(format!("Botão de controle GBC desconhecido: {button}")),
        };
        self.emu
            .runtime
            .cpu
            .clock
            .bus
            .io
            .joypad
            .handle(button, pressed);
        Ok(())
    }

    /// Reinicia com a ROM original, preservando o SRAM de bateria.
    pub fn restart(&mut self) {
        let save_ram = self.emu.runtime.cpu.clock.bus.cart.dump_ram();
        self.load_cart(save_ram);
    }

    /// Save state versionado: `EMMUGBC1` + JSON serializado do estado do
    /// oxGBC (inclui SRAM/MBC e RTC), compatível com o blob do IndexedDB.
    pub fn save_state(&mut self) -> Result<Vec<u8>, String> {
        let state: EmuSaveState = self.emu.create_save_state();
        let json = serde_json::to_vec(&state)
            .map_err(|error| format!("Não foi possível salvar o estado do GBC: {error}"))?;
        let mut bytes = Vec::with_capacity(STATE_MAGIC.len() + json.len());
        bytes.extend_from_slice(STATE_MAGIC);
        bytes.extend_from_slice(&json);
        Ok(bytes)
    }

    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), String> {
        if !bytes.starts_with(STATE_MAGIC) {
            return Err("Save state GBC inválido ou de outro console.".to_owned());
        }
        let state: EmuSaveState = serde_json::from_slice(&bytes[STATE_MAGIC.len()..])
            .map_err(|error| format!("Não foi possível carregar o estado do GBC: {error}"))?;
        self.emu.load_save_state(state);
        self.audio.samples.clear();
        Ok(())
    }

    /// SRAM de bateria bruto, para persistência fora do save state.
    pub fn save_ram(&self) -> Option<Vec<u8>> {
        self.emu
            .runtime
            .cpu
            .clock
            .bus
            .cart
            .dump_ram()
            .map(|bytes| bytes.to_vec())
    }

    pub fn load_save_ram(&mut self, bytes: &[u8]) {
        self.emu
            .runtime
            .cpu
            .clock
            .bus
            .cart
            .load_ram(bytes.to_vec().into_boxed_slice());
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub struct GbcEmulator {
        core: GbcCore,
    }

    #[wasm_bindgen]
    impl GbcEmulator {
        #[wasm_bindgen(constructor)]
        pub fn new(rom: &[u8]) -> Result<GbcEmulator, JsValue> {
            Ok(Self {
                core: GbcCore::new(rom).map_err(|error| JsValue::from_str(&error))?,
            })
        }

        #[wasm_bindgen(js_name = gameTitle)]
        pub fn game_title(&self) -> String {
            self.core.game_title().to_owned()
        }

        #[wasm_bindgen(js_name = runFrame)]
        pub fn run_frame(&mut self, render_graphics: bool) -> Result<Vec<u8>, JsValue> {
            self.core
                .run_frame(render_graphics)
                .map_err(|error| JsValue::from_str(&error))
        }

        #[wasm_bindgen(js_name = audioSamples)]
        pub fn audio_samples(&mut self) -> Vec<f32> {
            self.core.take_audio_samples()
        }

        #[wasm_bindgen(js_name = audioSampleRate)]
        pub fn audio_sample_rate(&self) -> u32 {
            self.core.audio_sample_rate()
        }

        #[wasm_bindgen(js_name = audioChannels)]
        pub fn audio_channels(&self) -> u32 {
            self.core.audio_channels()
        }

        #[wasm_bindgen(js_name = setButton)]
        pub fn set_button(
            &mut self,
            button: &str,
            pressed: bool,
            player: u32,
        ) -> Result<(), JsValue> {
            self.core
                .set_button(button, pressed, player as u8)
                .map_err(|error| JsValue::from_str(&error))
        }

        #[wasm_bindgen(js_name = setVolume)]
        pub fn set_volume(&mut self, volume: f64) {
            self.core.set_volume(volume);
        }

        pub fn restart(&mut self) {
            self.core.restart();
        }

        #[wasm_bindgen(js_name = saveState)]
        pub fn save_state(&mut self) -> Result<Vec<u8>, JsValue> {
            self.core
                .save_state()
                .map_err(|error| JsValue::from_str(&error))
        }

        #[wasm_bindgen(js_name = loadState)]
        pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
            self.core
                .load_state(bytes)
                .map_err(|error| JsValue::from_str(&error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_rom() -> Vec<u8> {
        // ROMOnly com cabeçalho válido, aceito pelo oxGBC.
        vec![0u8; 0x8000]
    }

    #[test]
    fn runs_frame_and_returns_rgba() {
        let mut core = GbcCore::new(&minimal_rom()).expect("ROM mínima deve carregar");
        let frame = core.run_frame(true).expect("deve avançar um frame");
        assert_eq!(frame.len(), FRAME_WIDTH * FRAME_HEIGHT * 4);
        assert_eq!(frame[3], 0xFF);
    }

    #[test]
    fn save_state_round_trip() {
        let mut core = GbcCore::new(&minimal_rom()).unwrap();
        core.run_frame(true).unwrap();
        core.set_button("A", true, 1).unwrap();
        core.run_frame(true).unwrap();

        let state = core.save_state().unwrap();
        assert!(state.starts_with(STATE_MAGIC));

        core.set_button("A", false, 1).unwrap();
        core.load_state(&state).unwrap();
        assert!(core.load_state(b"ERRADO").is_err());
    }

    #[test]
    fn restarts_and_keeps_audio_api() {
        let mut core = GbcCore::new(&minimal_rom()).unwrap();
        core.run_frame(true).unwrap();
        assert_eq!(core.audio_sample_rate(), 44_100);
        assert_eq!(core.audio_channels(), 2);
        core.restart();
        assert!(core.run_frame(false).unwrap().is_empty());
    }

    #[test]
    fn real_rom_smoke_test() {
        let path = match std::env::var("EMMU_GBC_TEST_ROM") {
            Ok(path) if !path.is_empty() => path,
            _ => return,
        };
        let rom = std::fs::read(&path).expect("ROM de teste não encontrada");
        let mut core = GbcCore::new(&rom).expect("ROM real deve carregar");
        let mut last_frame = Vec::new();
        for _ in 0..90 {
            last_frame = core.run_frame(true).expect("frame deve ser gerado");
            assert_eq!(last_frame.len(), FRAME_WIDTH * FRAME_HEIGHT * 4);
            let _audio = core.take_audio_samples();
        }
        // Um jogo real deve ter produzido imagem com mais de uma cor.
        assert!(
            last_frame
                .chunks_exact(4)
                .any(|pixel| pixel != &[0, 0, 0, 255]),
            "frame de {path} está inteiro preto"
        );
        assert!(
            last_frame
                .chunks_exact(4)
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 1,
            "frame de {path} tem uma única cor"
        );
    }
}
