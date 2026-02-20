mod audio;
mod config;
mod overlay;
mod paste;
mod transcribe;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use device_query::{DeviceQuery, DeviceState, Keycode};
use overlay::{AppState, STATUS_IDLE, STATUS_RECORDING, STATUS_RESULT, STATUS_TRANSCRIBING};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // CLI commands
    if args.len() > 1 {
        match args[1].as_str() {
            "--setup" | "setup" => {
                config::setup_interactive();
                return;
            }
            "--api-key" | "set-key" => {
                let key = args.get(2).unwrap_or_else(|| {
                    eprintln!("Usage: whisper-paste --api-key <your-key>");
                    std::process::exit(1);
                });
                match config::save_api_key(key) {
                    Ok(()) => {
                        println!("API key saved to {}", config::config_path().display());
                        println!("Run `whisper-paste` to start.");
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            "--help" | "-h" => {
                println!("whisper-paste - voice to text, pasted anywhere");
                println!();
                println!("Usage:");
                println!("  whisper-paste              Start with overlay UI");
                println!("  whisper-paste --no-ui      Start without overlay (terminal only)");
                println!("  whisper-paste --setup      Interactive setup (save API key)");
                println!("  whisper-paste --api-key K  Save API key directly");
                println!("  whisper-paste --help       Show this help");
                return;
            }
            "--no-ui" => {
                run_headless();
                return;
            }
            other => {
                eprintln!("Unknown option: {other}");
                eprintln!("Run `whisper-paste --help` for usage.");
                std::process::exit(1);
            }
        }
    }

    run_with_overlay();
}

fn run_with_overlay() {
    let cfg = config::load_config();
    let state = Arc::new(AppState::new());

    println!("whisper-paste running (with overlay)");
    println!("  Hotkey: Ctrl+Shift+R");
    println!("  Ctrl+C to quit");

    // Spawn hotkey + recording logic on background thread
    let state_clone = state.clone();
    std::thread::spawn(move || {
        hotkey_loop(cfg, state_clone);
    });

    // Run GUI on main thread (required on macOS)
    let app = overlay::OverlayApp::new(state);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([420.0, 48.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_position([490.0, 24.0]),
        ..Default::default()
    };

    eframe::run_native(
        "whisper-paste",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .expect("failed to run overlay");
}

fn run_headless() {
    let cfg = config::load_config();
    let state = Arc::new(AppState::new());

    println!("whisper-paste running (no UI)");
    println!("  Hotkey: Ctrl+Shift+R");
    println!("  Ctrl+C to quit");

    hotkey_loop(cfg, state);
}

fn hotkey_loop(cfg: config::Config, state: Arc<AppState>) {
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime"),
    );

    let device_state = DeviceState::new();
    let mut hotkey_held = false;
    let mut last_toggle = Instant::now();

    loop {
        let keys = device_state.get_keys();
        let hotkey_pressed = keys.contains(&Keycode::LControl)
            && keys.contains(&Keycode::LShift)
            && keys.contains(&Keycode::R);

        if hotkey_pressed && !hotkey_held && last_toggle.elapsed() > Duration::from_millis(500) {
            hotkey_held = true;
            last_toggle = Instant::now();

            let status = state.status.load(Ordering::Relaxed);

            if status == STATUS_TRANSCRIBING {
                // Still transcribing, ignore
            } else if status == STATUS_IDLE || status == STATUS_RESULT {
                // Start recording (also from result state)
                state.status.store(STATUS_RECORDING, Ordering::SeqCst);
                state.stop_signal.store(false, Ordering::SeqCst);
                // Clear old waveform
                state.waveform.lock().unwrap().clear();

                let state_c = state.clone();
                let api_key = cfg.api_key.clone();
                let model = cfg.model.clone();
                let rt = rt.clone();

                std::thread::spawn(move || {
                    println!("Recording...");

                    let waveform = Arc::new(std::sync::Mutex::new(Vec::new()));

                    // Share waveform with overlay
                    {
                        let wf = waveform.clone();
                        let state_wf = state_c.clone();
                        std::thread::spawn(move || {
                            // Periodically copy waveform data to overlay state
                            while state_wf.status.load(Ordering::Relaxed) == STATUS_RECORDING {
                                {
                                    let src = wf.lock().unwrap();
                                    let mut dst = state_wf.waveform.lock().unwrap();
                                    dst.clear();
                                    dst.extend_from_slice(&src);
                                }
                                std::thread::sleep(Duration::from_millis(50));
                            }
                        });
                    }

                    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let stop_clone = stop.clone();

                    // Monitor the app state stop_signal
                    let state_stop = state_c.clone();
                    std::thread::spawn(move || {
                        while !state_stop.stop_signal.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(30));
                        }
                        stop_clone.store(true, Ordering::SeqCst);
                    });

                    match audio::record_until_stopped(stop, Some(waveform)) {
                        Ok(samples) => {
                            if samples.is_empty() {
                                println!("(no audio captured)");
                                state_c.status.store(STATUS_IDLE, Ordering::SeqCst);
                                return;
                            }

                            state_c.status.store(STATUS_TRANSCRIBING, Ordering::SeqCst);
                            println!("Transcribing...");
                            let wav = audio::samples_to_wav(&samples);

                            match rt.block_on(transcribe::transcribe(&api_key, &model, wav)) {
                                Ok(text) => {
                                    if text.is_empty() {
                                        println!("(no speech detected)");
                                        state_c.status.store(STATUS_IDLE, Ordering::SeqCst);
                                    } else {
                                        println!("Result: {}", text);
                                        // Store result for overlay display
                                        *state_c.last_result.lock().unwrap() = text.clone();
                                        // Try to paste
                                        if let Err(e) = paste::paste_text(&text) {
                                            eprintln!("paste error: {e}");
                                        }
                                        // Show result in overlay
                                        state_c.status.store(STATUS_RESULT, Ordering::SeqCst);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("transcription error: {e}");
                                    state_c.status.store(STATUS_IDLE, Ordering::SeqCst);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("recording error: {e}");
                            state_c.status.store(STATUS_IDLE, Ordering::SeqCst);
                        }
                    }
                });
            } else if status == STATUS_RECORDING {
                // Stop recording
                println!("Stopped recording.");
                state.stop_signal.store(true, Ordering::SeqCst);
            }
        }

        if !hotkey_pressed {
            hotkey_held = false;
        }

        std::thread::sleep(Duration::from_millis(30));
    }
}
