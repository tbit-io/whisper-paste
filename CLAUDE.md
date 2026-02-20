# whisper-paste

Voice-to-text transcription tool. Records voice, transcribes with OpenAI Whisper API, pastes result into any focused input.

## Architecture

Single Rust binary, no runtime. GUI runs on main thread (macOS requirement), hotkey polling + recording on background threads.

```
src/
  main.rs        Entry point, CLI arg parsing, hotkey loop, state machine
  audio.rs       cpal recording, mono downmix, linear resampling to 16kHz, WAV encoding (hound)
  config.rs      Config load/save from platform dirs, env var fallback (OPENAI_API_KEY)
  transcribe.rs  OpenAI Whisper API call (reqwest multipart POST)
  paste.rs       Clipboard (arboard) + platform paste simulation
  overlay.rs     eframe/egui floating overlay — glass style, waveform, drag, fade, auto-hide
```

### State machine

`AppState.status` is an `AtomicU8` shared between threads:

```
IDLE (0) ──hotkey──> RECORDING (1) ──hotkey/stop──> TRANSCRIBING (2) ──done──> RESULT (3) ──fade──> IDLE
```

### Platform paste

- **macOS**: `osascript` keystroke Cmd+V (avoids enigo thread crash)
- **Linux**: `xdotool` (X11) or `ydotool` (Wayland)
- **Windows**: `enigo` Ctrl+V

### Audio pipeline

1. Query device native config (`default_input_config`)
2. Record in native format (any sample rate, any channels)
3. Downmix to mono (average channels)
4. Linear interpolation resample to 16kHz
5. Encode to 16-bit PCM WAV (hound)
6. POST to OpenAI `/v1/audio/transcriptions`

## Build & run

```sh
cargo build              # dev
cargo build --release    # optimized (LTO + strip)
cargo install --path .   # install to ~/.cargo/bin
cargo test               # 11 unit tests (audio + config)
```

## Key dependencies

| Crate | Purpose |
|-------|---------|
| cpal 0.15 | Cross-platform audio input |
| hound 3.5 | WAV encoding |
| eframe 0.30 | GUI overlay (egui) |
| reqwest 0.12 | HTTP client (rustls-tls, no OpenSSL) |
| device_query 2 | Global hotkey polling |
| arboard 3 | Clipboard |
| enigo 0.3 | Keyboard sim (Windows only) |

## Lessons learned

### macOS thread safety
`enigo` calls HIToolbox APIs (`TSMGetInputSourceProperty`) which must run on the main thread dispatch queue. Using enigo from a background thread causes `EXC_BREAKPOINT`. Solution: use `osascript` for Cmd+V on macOS instead.

### Audio device config
Never force 16kHz/mono on the input device — many devices (especially macOS) don't support it. Always query `default_input_config()` and resample in software.

### eframe 0.30 API
- `Frame::none()` not `Frame::NONE`
- `rounding` not `corner_radius`
- `Margin { left, right, top, bottom }` — no `Margin::symmetric`
- `clear_color()` override needed for true transparency

### HiDPI drag
`drag_delta()` returns UI-coordinate deltas. Divide by `pixels_per_point()` to get screen-pixel deltas for `ViewportCommand::OuterPosition`.

### Overlay drag during recording
Using `ui.allocate_exact_size(vec2(w, 0.0), Sense::drag())` doesn't register drags because the rect has zero height. Use `ui.interact(full_rect, ...)` over the entire panel area instead.

## Changelog

### v0.1.0 — Initial release
- Global hotkey (Ctrl+Shift+R) to start/stop recording
- OpenAI Whisper API transcription
- Auto-paste into focused input (clipboard + simulated Cmd/Ctrl+V)
- Floating glass-style overlay with:
  - Live waveform visualization during recording
  - Bouncing dots animation while transcribing
  - Result display with Copy button
  - Clickable Stop button
  - Drag support in all states (remembers position)
  - Fade in/out animation
  - Auto-hide after 3s idle (6s for results)
- CLI onboarding: `--setup`, `--api-key`, `--help`
- Headless mode: `--no-ui`
- Cross-platform: macOS, Linux, Windows
- 11 unit tests (WAV encoding, resampling, config)
- GitHub Actions release workflow for pre-built binaries
