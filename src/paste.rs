use arboard::Clipboard;
use std::thread;
use std::time::Duration;

pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clip = Clipboard::new().map_err(|e| format!("clipboard error: {e}"))?;
    clip.set_text(text).map_err(|e| format!("clipboard set error: {e}"))?;

    thread::sleep(Duration::from_millis(100));

    simulate_paste()
}

#[cfg(target_os = "macos")]
fn simulate_paste() -> Result<(), String> {
    // Use osascript to simulate Cmd+V — safe from any thread
    std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .output()
        .map_err(|e| format!("osascript failed: {e}"))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn simulate_paste() -> Result<(), String> {
    // Try xdotool first, fall back to xclip hint
    let result = std::process::Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .output();

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            // Try ydotool for Wayland
            std::process::Command::new("ydotool")
                .args(["key", "29:1", "47:1", "47:0", "29:0"])
                .output()
                .map_err(|_| "install xdotool (X11) or ydotool (Wayland) to auto-paste".to_string())?;
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
fn simulate_paste() -> Result<(), String> {
    use enigo::{Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("enigo error: {e}"))?;
    enigo
        .key(Key::Control, enigo::Direction::Press)
        .map_err(|e| format!("key error: {e}"))?;
    enigo
        .key(Key::Unicode('v'), enigo::Direction::Click)
        .map_err(|e| format!("key error: {e}"))?;
    enigo
        .key(Key::Control, enigo::Direction::Release)
        .map_err(|e| format!("key error: {e}"))?;
    Ok(())
}
