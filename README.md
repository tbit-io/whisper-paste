# whisper-paste

Record your voice, transcribe it with OpenAI Whisper, and paste the result into any input on your computer.

Works on **Linux**, **macOS**, and **Windows**. Single binary, no runtime needed.

## Install

### Quick install (curl)

**macOS (Apple Silicon):**
```sh
curl -fsSL https://github.com/tbit-io/whisper-paste/releases/latest/download/whisper-paste-macos-aarch64 -o whisper-paste \
  && chmod +x whisper-paste && sudo mv whisper-paste /usr/local/bin/
```

**macOS (Intel):**
```sh
curl -fsSL https://github.com/tbit-io/whisper-paste/releases/latest/download/whisper-paste-macos-x86_64 -o whisper-paste \
  && chmod +x whisper-paste && sudo mv whisper-paste /usr/local/bin/
```

**Linux (x86_64):**
```sh
curl -fsSL https://github.com/tbit-io/whisper-paste/releases/latest/download/whisper-paste-linux-x86_64 -o whisper-paste \
  && chmod +x whisper-paste && sudo mv whisper-paste /usr/local/bin/
```

**Linux (aarch64):**
```sh
curl -fsSL https://github.com/tbit-io/whisper-paste/releases/latest/download/whisper-paste-linux-aarch64 -o whisper-paste \
  && chmod +x whisper-paste && sudo mv whisper-paste /usr/local/bin/
```

**Windows (PowerShell):**
```powershell
Invoke-WebRequest -Uri https://github.com/tbit-io/whisper-paste/releases/latest/download/whisper-paste-windows-x86_64.exe -OutFile whisper-paste.exe
```

### Quick install (wget)

```sh
wget -qO whisper-paste https://github.com/tbit-io/whisper-paste/releases/latest/download/whisper-paste-linux-x86_64 \
  && chmod +x whisper-paste && sudo mv whisper-paste /usr/local/bin/
```

### From source

```sh
git clone https://github.com/tbit-io/whisper-paste.git
cd whisper-paste
cargo install --path .
```

### From crates.io (coming soon)

```sh
cargo install whisper-paste
```

## Quick start

```sh
# 1. Set up your OpenAI API key
whisper-paste --setup

# 2. Start
whisper-paste
```

1. Press **Ctrl+Shift+R** to start recording
2. Press **Ctrl+Shift+R** again (or click **Stop**) to stop
3. The transcription is pasted into whatever input has focus

A floating overlay shows recording status with a live waveform, transcribing animation, and the result text. The overlay is draggable, auto-hides when idle, and remembers its position.

## Configuration

Your API key is stored in a config file:

| OS      | Path                                              |
|---------|----------------------------------------------------|
| Linux   | `~/.config/whisper-paste/config.toml`              |
| macOS   | `~/Library/Application Support/whisper-paste/config.toml` |
| Windows | `%APPDATA%\whisper-paste\config.toml`              |

You can also use the `OPENAI_API_KEY` environment variable (takes priority over config file).

```toml
api_key = "sk-your-key-here"

# optional, defaults to "whisper-1"
# model = "whisper-1"
```

## CLI

```
whisper-paste              Start with overlay UI
whisper-paste --no-ui      Start without overlay (terminal only)
whisper-paste --setup      Interactive setup (save API key)
whisper-paste --api-key K  Save API key directly
whisper-paste --help       Show help
```

## Platform notes

- **macOS**: Grant microphone + accessibility permissions to the terminal/binary
- **Linux**: Needs ALSA (`libasound2-dev`) or PulseAudio dev libs to build. Needs `xdotool` (X11) or `ydotool` (Wayland) for auto-paste
- **Windows**: Works out of the box

## Contributing

```sh
git clone https://github.com/tbit-io/whisper-paste.git
cd whisper-paste
cargo build
cargo test
```

See [CLAUDE.md](CLAUDE.md) for architecture details, lessons learned, and development context.

## License

MIT
