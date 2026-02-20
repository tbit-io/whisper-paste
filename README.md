# whisper-paste

Record your voice, transcribe it with OpenAI Whisper, and paste the result into any input on your computer.

Works on **Linux**, **macOS**, and **Windows**. Single binary, no runtime needed.

## Install

Download the binary from [Releases](https://github.com/tbit-io/whisper-paste/releases) and put it in your PATH.

Or build from source:

```sh
cargo install --path .
```

## Quick start

```sh
# Interactive setup — prompts for your OpenAI API key
whisper-paste --setup

# Or set the key directly
whisper-paste --api-key sk-your-key-here

# Start
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
- **Linux**: Needs ALSA (`libasound2-dev`) or PulseAudio dev libs to build
- **Windows**: Works out of the box

## License

MIT
