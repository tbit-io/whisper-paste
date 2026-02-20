use eframe::egui;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    /// 0 = idle, 1 = recording, 2 = transcribing, 3 = result
    pub status: AtomicU8,
    pub waveform: Mutex<Vec<f32>>,
    pub stop_signal: std::sync::atomic::AtomicBool,
    /// Last transcription result for display
    pub last_result: Mutex<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(0),
            waveform: Mutex::new(Vec::new()),
            stop_signal: std::sync::atomic::AtomicBool::new(false),
            last_result: Mutex::new(String::new()),
        }
    }
}

pub const STATUS_IDLE: u8 = 0;
pub const STATUS_RECORDING: u8 = 1;
pub const STATUS_TRANSCRIBING: u8 = 2;
pub const STATUS_RESULT: u8 = 3;

pub struct OverlayApp {
    pub state: Arc<AppState>,
    phase: f32,
    last_status: u8,
    idle_since: Option<std::time::Instant>,
    /// 0.0 = hidden, 1.0 = fully visible — for fade animation
    opacity: f32,
    target_opacity: f32,
    /// Remember last position so overlay reappears in the same spot
    saved_position: Option<egui::Pos2>,
}

impl OverlayApp {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            phase: 0.0,
            last_status: STATUS_IDLE,
            idle_since: Some(std::time::Instant::now()),
            opacity: 1.0,
            target_opacity: 1.0,
            saved_position: None,
        }
    }
}

// Glass-style colors with transparency
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(160, 160, 166);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(240, 240, 245);
const RED: egui::Color32 = egui::Color32::from_rgb(255, 69, 58);
const GREEN: egui::Color32 = egui::Color32::from_rgb(48, 209, 88);
const BLUE: egui::Color32 = egui::Color32::from_rgb(10, 132, 255);

const ROUNDING: f32 = 18.0;
const IDLE_HIDE_DELAY: f64 = 3.0;
const FADE_SPEED: f32 = 0.08; // per frame

fn glass_bg(opacity: f32) -> egui::Color32 {
    let a = (opacity * 180.0) as u8; // semi-transparent dark
    egui::Color32::from_rgba_unmultiplied(25, 25, 28, a)
}

fn glass_border(opacity: f32) -> egui::Color32 {
    let a = (opacity * 80.0) as u8;
    egui::Color32::from_rgba_unmultiplied(255, 255, 255, a)
}

fn bar_bg(opacity: f32) -> egui::Color32 {
    let a = (opacity * 120.0) as u8;
    egui::Color32::from_rgba_unmultiplied(255, 255, 255, a)
}

fn with_opacity(c: egui::Color32, opacity: f32) -> egui::Color32 {
    let [r, g, b, a] = c.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, (a as f32 * opacity) as u8)
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let status = self.state.status.load(Ordering::Relaxed);

        // Track status transitions
        if status != self.last_status {
            if status == STATUS_IDLE {
                self.idle_since = Some(std::time::Instant::now());
            } else if status == STATUS_RESULT {
                // Show result for 6 seconds before fading
                self.idle_since = Some(std::time::Instant::now());
                self.target_opacity = 1.0;
            } else {
                self.idle_since = None;
                self.target_opacity = 1.0;
                // Restore saved position and full size when becoming active
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(420.0, 48.0)));
                if let Some(pos) = self.saved_position {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
                }
            }
            self.last_status = status;
        }

        // Determine target opacity
        let hide_delay = if status == STATUS_RESULT {
            6.0 // show result longer
        } else {
            IDLE_HIDE_DELAY
        };

        let should_hide = if let Some(since) = self.idle_since {
            since.elapsed().as_secs_f64() > hide_delay
        } else {
            false
        };

        if should_hide && (status == STATUS_IDLE || status == STATUS_RESULT) {
            self.target_opacity = 0.0;
            // Transition result -> idle when faded
            if status == STATUS_RESULT && self.opacity < 0.05 {
                self.state.status.store(STATUS_IDLE, Ordering::SeqCst);
            }
        }

        // Animate opacity
        if (self.opacity - self.target_opacity).abs() > 0.01 {
            if self.opacity < self.target_opacity {
                self.opacity = (self.opacity + FADE_SPEED).min(1.0);
            } else {
                self.opacity = (self.opacity - FADE_SPEED * 0.6).max(0.0); // slower fade out
            }
        } else {
            self.opacity = self.target_opacity;
        }

        // Always animate during transitions
        if (self.opacity - self.target_opacity).abs() > 0.01 || status != STATUS_IDLE {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            if status != STATUS_IDLE {
                self.phase += 0.05;
            }
        } else if self.opacity > 0.01 {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        } else {
            // Fully hidden — check less often
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }

        // Save current position while visible (for restoring later)
        if self.opacity > 0.1 {
            self.saved_position = ctx.input(|i| i.viewport().outer_rect.map(|r| r.left_top()));
        }

        // When fully hidden, show a tiny clickable dot
        if self.opacity < 0.05 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(24.0, 24.0)));

            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                .show(ctx, |ui: &mut egui::Ui| {
                    let (rect, resp) = ui.allocate_exact_size(
                        ui.available_size(),
                        egui::Sense::click_and_drag(),
                    );

                    // Tiny subtle dot
                    ui.painter().circle_filled(
                        rect.center(),
                        4.0,
                        egui::Color32::from_rgba_unmultiplied(160, 160, 166, 100),
                    );

                    handle_drag(ctx, &resp);

                    if resp.clicked() || resp.hovered() {
                        self.idle_since = None;
                        self.target_opacity = 1.0;
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                            egui::vec2(420.0, 48.0),
                        ));
                        // Restore saved position
                        if let Some(pos) = self.saved_position {
                            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
                        }
                    }
                });
            return;
        }

        // Ensure full size when visible
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(420.0, 48.0)));

        let op = self.opacity;

        let panel_frame = egui::Frame::none()
            .fill(glass_bg(op))
            .rounding(ROUNDING)
            .stroke(egui::Stroke::new(0.5, glass_border(op)))
            .inner_margin(egui::Margin {
                left: 16.0,
                right: 16.0,
                top: 10.0,
                bottom: 10.0,
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui: &mut egui::Ui| {
                // Full-size background for drag (covers entire panel)
                let full_rect = ui.max_rect();
                let drag_resp = ui.interact(full_rect, ui.id().with("drag"), egui::Sense::drag());
                handle_drag(ctx, &drag_resp);

                panel_frame.show(ui, |ui: &mut egui::Ui| {
                    ui.horizontal_centered(|ui: &mut egui::Ui| {
                        match status {
                            STATUS_RECORDING => {
                                draw_recording(ui, &self.state, self.phase, op);
                            }
                            STATUS_TRANSCRIBING => {
                                draw_transcribing(ui, self.phase, op);
                            }
                            STATUS_RESULT => {
                                draw_result(ui, &self.state, op, &mut self.idle_since);
                            }
                            _ => {
                                draw_idle(ui, &mut self.idle_since, op);
                            }
                        }
                    });
                });
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn handle_drag(ctx: &egui::Context, resp: &egui::Response) {
    if resp.dragged() {
        let delta = resp.drag_delta();
        if delta.length() > 0.0 {
            // Account for HiDPI scale factor so drag matches cursor 1:1
            let ppp = ctx.pixels_per_point();
            let screen_pos = ctx.input(|i| {
                i.viewport()
                    .outer_rect
                    .map_or(egui::pos2(0.0, 0.0), |r| r.left_top())
            });
            let new_pos = egui::pos2(
                screen_pos.x + delta.x / ppp,
                screen_pos.y + delta.y / ppp,
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(new_pos));
        }
    }
}

fn draw_idle(ui: &mut egui::Ui, idle_since: &mut Option<std::time::Instant>, op: f32) {
    // Mic icon
    let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    let dim = with_opacity(TEXT_DIM, op);
    ui.painter().circle_filled(icon_rect.center(), 4.0, dim);
    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(icon_rect.center().x - 1.5, icon_rect.center().y + 3.0),
            egui::vec2(3.0, 4.0),
        ),
        0.0,
        dim,
    );

    ui.add_space(6.0);

    ui.label(
        egui::RichText::new("Ready  |  Ctrl+Shift+R to record")
            .color(with_opacity(TEXT_DIM, op))
            .size(12.0),
    );

    if ui.ui_contains_pointer() {
        *idle_since = Some(std::time::Instant::now());
    }
}

fn draw_recording(ui: &mut egui::Ui, state: &Arc<AppState>, phase: f32, op: f32) {
    // Pulsing red dot
    let pulse = (phase * 3.0).sin() * 0.3 + 0.7;
    let red_a = (pulse * 255.0 * op) as u8;
    let pulsing_red = egui::Color32::from_rgba_unmultiplied(255, 69, 58, red_a);

    let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().circle_filled(dot_rect.center(), 4.5, pulsing_red);
    ui.painter().circle_filled(dot_rect.center(), 3.0, with_opacity(RED, op));

    ui.add_space(8.0);

    // Waveform
    let waveform_width = 140.0;
    let waveform_height = 22.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(waveform_width, waveform_height), egui::Sense::hover());

    let samples = state.waveform.lock().unwrap();
    let n_bars: usize = 28;
    let bar_width = waveform_width / n_bars as f32;
    let gap = 1.2;

    for i in 0..n_bars {
        let bar_h = if !samples.is_empty() {
            let chunk_size = samples.len().max(1) / n_bars.max(1);
            let start = i * chunk_size;
            let end = (start + chunk_size).min(samples.len());
            if start < samples.len() && end > start {
                let rms: f32 = {
                    let sum: f32 = samples[start..end].iter().map(|s| s * s).sum();
                    (sum / (end - start) as f32).sqrt()
                };
                (rms * waveform_height * 6.0).clamp(2.0, waveform_height - 2.0)
            } else {
                2.0
            }
        } else {
            ((phase * 2.0 + i as f32 * 0.25).sin() * 0.4 + 0.5) * waveform_height * 0.4 + 2.0
        };

        let x = rect.left() + i as f32 * bar_width;
        let cy = rect.center().y;

        // Background track
        let bg_rect = egui::Rect::from_min_max(
            egui::pos2(x + gap, cy - waveform_height / 2.0 + 1.0),
            egui::pos2(x + bar_width - gap, cy + waveform_height / 2.0 - 1.0),
        );
        ui.painter().rect_filled(bg_rect, 2.0, bar_bg(op));

        // Active bar
        let active_rect = egui::Rect::from_min_max(
            egui::pos2(x + gap, cy - bar_h / 2.0),
            egui::pos2(x + bar_width - gap, cy + bar_h / 2.0),
        );
        ui.painter().rect_filled(active_rect, 2.0, with_opacity(GREEN, op));
    }

    ui.add_space(8.0);

    ui.label(
        egui::RichText::new("Recording")
            .color(with_opacity(TEXT_PRIMARY, op))
            .size(12.0),
    );

    ui.add_space(6.0);

    // Clickable stop button
    let stop_resp = ui.add(
        egui::Button::new(
            egui::RichText::new("  Stop  ")
                .color(with_opacity(egui::Color32::WHITE, op))
                .size(10.0),
        )
        .fill(with_opacity(RED, op * 0.8))
        .rounding(8.0)
        .stroke(egui::Stroke::NONE),
    );

    if stop_resp.clicked() {
        state.stop_signal.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

fn draw_result(
    ui: &mut egui::Ui,
    state: &Arc<AppState>,
    op: f32,
    idle_since: &mut Option<std::time::Instant>,
) {
    // Checkmark
    ui.label(
        egui::RichText::new("✓")
            .color(with_opacity(GREEN, op))
            .size(14.0),
    );

    ui.add_space(6.0);

    // Show truncated result text
    let result = state.last_result.lock().unwrap();
    let display_text = if result.len() > 50 {
        format!("{}...", &result[..50])
    } else {
        result.clone()
    };

    ui.label(
        egui::RichText::new(&display_text)
            .color(with_opacity(TEXT_PRIMARY, op))
            .size(11.0),
    );

    ui.add_space(6.0);

    // Copy button
    let copy_resp = ui.add(
        egui::Button::new(
            egui::RichText::new(" Copy ")
                .color(with_opacity(egui::Color32::WHITE, op))
                .size(10.0),
        )
        .fill(with_opacity(BLUE, op * 0.8))
        .rounding(8.0)
        .stroke(egui::Stroke::NONE),
    );

    if copy_resp.clicked() {
        if let Ok(mut clip) = arboard::Clipboard::new() {
            clip.set_text(result.clone()).ok();
        }
    }

    // Reset idle timer on hover
    if ui.ui_contains_pointer() {
        *idle_since = Some(std::time::Instant::now());
    }
}

fn draw_transcribing(ui: &mut egui::Ui, phase: f32, op: f32) {
    // Bouncing dots
    let n_dots = 3;
    let dot_spacing = 10.0;
    let total_w = n_dots as f32 * dot_spacing + 4.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_w, 14.0), egui::Sense::hover());

    for i in 0..n_dots {
        let t = (phase * 2.5 - i as f32 * 0.5).sin() * 0.5 + 0.5;
        let size = 2.5 + t * 1.5;
        let x = rect.left() + i as f32 * dot_spacing + dot_spacing / 2.0 + 2.0;
        let y = rect.center().y - t * 3.0;
        ui.painter()
            .circle_filled(egui::pos2(x, y), size, with_opacity(BLUE, op));
    }

    ui.add_space(8.0);

    ui.label(
        egui::RichText::new("Transcribing...")
            .color(with_opacity(BLUE, op))
            .size(12.0),
    );
}
