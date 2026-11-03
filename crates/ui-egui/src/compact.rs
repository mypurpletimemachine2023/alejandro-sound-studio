//! Academy phone presentation. It never installs or saves a replacement desktop dock.
use crate::compact_policy::{Preference, next_compact};
use crate::{SoundApp, menus, panels};
use egui::{Pos2, Rect, TouchId, TouchPhase, Vec2, vec2};
use serde_json::json;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Drawer {
    #[default]
    None,
    Tracks,
    Clips,
    Mixer,
}

pub struct Runtime {
    pub active: bool,
    viewport: Rect,
    controls: std::collections::BTreeMap<&'static str, [f32; 4]>,
    navigate: bool,
    drawer: Drawer,
    body: Rect,
    touches: Vec<(TouchId, Pos2)>,
    captured: bool,
    mouse_capture: bool,
    blocked: bool,
    center: Option<Pos2>,
    span: Option<f32>,
    checkpoint: Option<(u64, soundcraft_engine::InteractionCheckpoint)>,
    release_pending: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            active: false,
            viewport: Rect::NOTHING,
            controls: std::collections::BTreeMap::new(),
            navigate: false,
            drawer: Drawer::None,
            body: Rect::NOTHING,
            touches: Vec::new(),
            captured: false,
            mouse_capture: false,
            blocked: false,
            center: None,
            span: None,
            checkpoint: None,
            release_pending: false,
        }
    }
}

impl Runtime {
    /// Presentation-only evidence: no document contents, commands or mutation are exposed.
    pub fn presentation_state(&self, preferred: &str) -> serde_json::Value {
        let rect = |r: Rect| [r.min.x, r.min.y, r.max.x, r.max.y];
        json!({
            "version": 1,
            "applied": if self.active { "compact" } else { "desktop" },
            "preferred": Preference::parse(preferred).id(),
            "input": if self.navigate { "navigate" } else { "edit" },
            "pane": match self.drawer { Drawer::None => "session", Drawer::Tracks => "track-list", Drawer::Clips => "clips", Drawer::Mixer => "mixer" },
            "viewport": rect(self.viewport),
            "canvas": if self.body.is_finite() { Some(rect(self.body)) } else { None },
            "controls": self.controls,
            "contacts": self.touches.len(),
            "gesturePending": self.gesture_pending()
        })
    }

    pub fn gesture_pending(&self) -> bool {
        self.checkpoint.is_some() || !self.touches.is_empty() || self.mouse_capture || self.release_pending
    }

    pub(crate) fn end_ui_frame(&mut self) {
        self.release_pending = false;
    }

    pub fn document_replaced(&mut self) {
        self.checkpoint = None;
        self.touches.clear();
        self.captured = false;
        self.mouse_capture = false;
        self.blocked = false;
        self.center = None;
        self.span = None;
        self.body = Rect::NOTHING;
        self.release_pending = false;
    }
}

pub(crate) fn begin_frame(app: &mut SoundApp, ui: &egui::Ui) {
    app.docking_context = Some(ui.ctx().clone());
    app.compact.viewport = ui.ctx().content_rect();
    let size = ui.ctx().content_rect().size();
    let busy = app.gesture.is_some() || ui.input(|i| i.pointer.any_down()) || !app.compact.touches.is_empty();
    app.compact.active = next_compact(app.compact.active, Preference::parse(&app.ui.academy_layout), size.x, size.y, busy);
}

fn cancel(app: &mut SoundApp, ctx: &egui::Context) {
    app.compact.release_pending = true;
    if let Some((epoch, checkpoint)) = app.compact.checkpoint.take()
        && epoch == app.doc_epoch
    {
        app.engine.cancel_interaction(checkpoint);
    }
    app.gesture = None;
    app.engine.end_merge();
    app.pointer_down = false;
    ctx.stop_dragging();
    ctx.input_mut(|i| i.pointer = Default::default());
    egui::DragAndDrop::clear_payload(ctx);
}

/// Run before egui converts a release into a completed edit. A second finger owns navigation
/// until every contact is lifted; it cannot accidentally finish the first finger's trim/fader.
pub(crate) fn raw_input(app: &mut SoundApp, ctx: &egui::Context, raw: &mut egui::RawInput) {
    if !app.compact.active {
        return;
    }
    if app.dialogs.open.is_some() {
        if app.compact.checkpoint.is_some() {
            cancel(app, ctx);
        }
        app.compact.touches.clear();
        app.compact.captured = false;
        app.compact.mouse_capture = false;
        app.compact.blocked = false;
        app.compact.center = None;
        app.compact.span = None;
        return;
    }
    if !raw.focused && (app.compact.captured || !app.compact.touches.is_empty()) {
        cancel(app, ctx);
        app.compact.touches.clear();
        app.compact.captured = false;
        app.compact.mouse_capture = false;
        app.compact.blocked = false;
        app.compact.center = None;
        app.compact.span = None;
        raw.events.retain(|e| !pointer_event(e));
        return;
    }
    let mut suppress = app.compact.blocked;
    let mut pan = Vec2::ZERO;
    let mut zoom = 1.0_f32;
    // Mouse/pen cancellation uses the same checkpoint, and Navigate also works with a
    // single-pointer drag. Touch-generated pointer events are handled only by the contact path.
    if !raw.events.iter().any(|event| matches!(event, egui::Event::Touch { .. })) && app.compact.touches.is_empty() {
        for event in &raw.events {
            match event {
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, .. } => {
                    if capturable(app, ctx, *pos) {
                        app.compact.mouse_capture = true;
                        app.compact.captured = true;
                        app.compact.checkpoint = Some((app.doc_epoch, app.engine.interaction_checkpoint()));
                        app.compact.center = Some(*pos);
                        if app.compact.navigate && app.compact.drawer == Drawer::None {
                            cancel(app, ctx);
                            suppress = true;
                        }
                    }
                }
                egui::Event::PointerMoved(pos) if app.compact.mouse_capture && app.compact.navigate && app.compact.drawer == Drawer::None => {
                    if let Some(before) = app.compact.center {
                        pan += *pos - before;
                    }
                    app.compact.center = Some(*pos);
                    suppress = true;
                }
                egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: false, .. } if app.compact.mouse_capture => {
                    app.compact.release_pending = true;
                    suppress |= app.compact.navigate && app.compact.drawer == Drawer::None;
                    app.compact.checkpoint = None;
                    app.compact.mouse_capture = false;
                    app.compact.captured = false;
                    app.compact.center = None;
                }
                egui::Event::PointerGone if app.compact.mouse_capture => {
                    cancel(app, ctx);
                    app.compact.mouse_capture = false;
                    app.compact.captured = false;
                    app.compact.center = None;
                    suppress = true;
                }
                _ => {}
            }
        }
    }
    for event in &raw.events {
        let egui::Event::Touch { id, phase, pos, .. } = event else { continue };
        let previous = app.compact.touches.iter().position(|(other, _)| other == id);
        match phase {
            TouchPhase::Start => {
                app.compact.center = None;
                app.compact.span = None;
                if app.compact.touches.is_empty() {
                    app.compact.captured = capturable(app, ctx, *pos);
                    if app.compact.captured {
                        app.compact.checkpoint = Some((app.doc_epoch, app.engine.interaction_checkpoint()));
                    }
                }
                if previous.is_none() && app.compact.touches.len() < 8 {
                    app.compact.touches.push((*id, *pos));
                }
            }
            TouchPhase::Move => {
                if let Some(index) = previous
                    && let Some(contact) = app.compact.touches.get_mut(index)
                {
                    contact.1 = *pos;
                }
            }
            TouchPhase::End | TouchPhase::Cancel => {
                app.compact.center = None;
                app.compact.span = None;
                app.compact.touches.retain(|(other, _)| other != id);
                if *phase == TouchPhase::Cancel && app.compact.captured {
                    cancel(app, ctx);
                    app.compact.blocked = true;
                    suppress = true;
                }
            }
        }
        let navigation = app.compact.captured
            && (app.compact.touches.len() >= 2 || app.compact.blocked || app.compact.navigate && app.compact.drawer == Drawer::None);
        if navigation {
            if !app.compact.blocked {
                cancel(app, ctx);
                app.compact.blocked = true;
            }
            suppress = true;
            if app.compact.drawer == Drawer::None && !app.compact.touches.is_empty() {
                let count = app.compact.touches.len() as f32;
                let sum = app.compact.touches.iter().fold(Vec2::ZERO, |sum, (_, p)| sum + p.to_vec2());
                let center = (sum / count).to_pos2();
                let span = app.compact.touches.first().zip(app.compact.touches.get(1)).map(|(a, b)| a.1.distance(b.1));
                if let Some(before) = app.compact.center {
                    pan += center - before;
                }
                if let Some((before, after)) = app.compact.span.zip(span).filter(|(a, b)| *a > 8.0 && *b > 8.0) {
                    zoom *= (after / before).clamp(0.8, 1.25);
                }
                app.compact.center = Some(center);
                app.compact.span = span;
            }
        } else {
            app.compact.center = None;
            app.compact.span = None;
        }
        if app.compact.touches.is_empty() {
            // A normal release still reaches the existing commit path. A navigation release is
            // suppressed for this entire input batch, even after the last contact disappears.
            app.compact.checkpoint = None;
            app.compact.release_pending = true;
            app.compact.captured = false;
            app.compact.blocked = false;
            app.compact.center = None;
            app.compact.span = None;
        }
    }
    if app.compact.captured && !app.compact.blocked && !app.compact.touches.is_empty() && raw.events.iter().any(|e| matches!(e, egui::Event::PointerGone)) {
        cancel(app, ctx);
        app.compact.touches.clear();
        app.compact.captured = false;
        app.compact.center = None;
        app.compact.span = None;
        suppress = true;
    }
    if suppress {
        raw.events.retain(|e| !pointer_event(e));
    }
    if pan != Vec2::ZERO {
        let state = app.engine.session().edit.zoom;
        let to = (state.scroll as f64 - f64::from(pan.x) * state.samples_per_px).clamp(0.0, soundcraft_engine::cmd::MAX_POSITION as f64);
        let _ = app.engine.execute("view.scroll", &json!({"to": soundcraft_time::to_samples(to)}));
        app.edit_layout.scroll_by(-pan.y, app.compact.body.height());
    }
    if zoom.is_finite() && (zoom - 1.0).abs() > 0.001 {
        let _ = app.engine.execute("view.zoom_by", &json!({"factor": zoom, "x": (app.compact.body.width() * 0.5).max(0.0)}));
    }
}

fn capturable(app: &SoundApp, ctx: &egui::Context, pos: Pos2) -> bool {
    app.compact.body.contains(pos) && ctx.layer_id_at(pos).is_none_or(|layer| layer.order == egui::Order::Background)
}

fn pointer_event(event: &egui::Event) -> bool {
    matches!(event, egui::Event::PointerButton { .. } | egui::Event::PointerMoved(_) | egui::Event::PointerGone | egui::Event::MouseMoved(_))
}

fn button(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    ui.add_sized(vec2(width, 44.0), egui::Button::new(label))
}

fn preference_menu(app: &mut SoundApp, ui: &mut egui::Ui) {
    ui.label("Workspace");
    for (label, value) in [("Auto", "auto"), ("Compact", "compact"), ("Desktop", "desktop")] {
        if ui.add_sized(vec2(150.0, 44.0), egui::Button::new(label).selected(app.ui.academy_layout == value)).clicked() {
            app.ui.academy_layout = value.into();
            ui.ctx().request_repaint();
            ui.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, RawInput, TouchDeviceId, pos2};

    fn app() -> SoundApp {
        SoundApp::new(soundcraft_engine::Engine::default(), None, crate::Services::default())
    }

    fn touch(id: u64, phase: TouchPhase, pos: Pos2) -> Event {
        Event::Touch { device_id: TouchDeviceId(1), id: TouchId(id), phase, pos, force: None }
    }

    #[test]
    fn second_finger_restores_eager_content_and_undo_without_finishing_a_drag() {
        let mut app = app();
        let ctx = egui::Context::default();
        app.compact.active = true;
        app.compact.body = Rect::from_min_size(Pos2::ZERO, vec2(390.0, 700.0));
        let before = serde_json::to_value(app.engine.session()).unwrap();
        let mut first = RawInput { events: vec![touch(1, TouchPhase::Start, pos2(180.0, 250.0))], ..Default::default() };
        raw_input(&mut app, &ctx, &mut first);
        app.engine.execute("track.new", &json!({"count": 1})).unwrap();
        assert!(app.engine.is_dirty());
        let mut second = RawInput {
            events: vec![
                touch(2, TouchPhase::Start, pos2(280.0, 250.0)),
                Event::PointerButton { pos: pos2(180.0, 250.0), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() },
            ],
            ..Default::default()
        };
        raw_input(&mut app, &ctx, &mut second);
        assert_eq!(serde_json::to_value(app.engine.session()).unwrap(), before);
        assert!(!app.engine.is_dirty());
        assert!(!app.engine.can_undo());
        assert!(!app.engine.can_redo());
        assert!(app.gesture.is_none());
        assert!(!second.events.iter().any(pointer_event));
    }

    #[test]
    fn focus_loss_and_contact_cancel_restore_only_the_owned_document() {
        let mut app = app();
        let ctx = egui::Context::default();
        app.compact.active = true;
        app.compact.body = Rect::from_min_size(Pos2::ZERO, vec2(390.0, 700.0));
        let mut first = RawInput { events: vec![touch(1, TouchPhase::Start, pos2(180.0, 250.0))], ..Default::default() };
        raw_input(&mut app, &ctx, &mut first);
        app.engine.execute("track.new", &json!({"count": 1})).unwrap();
        let mut hidden = RawInput { focused: false, ..Default::default() };
        raw_input(&mut app, &ctx, &mut hidden);
        assert!(app.engine.session().tracks.is_empty());
        assert!(!app.engine.is_dirty());
        assert!(app.compact.touches.is_empty());
        let mut first = RawInput { events: vec![touch(1, TouchPhase::Start, pos2(180.0, 250.0))], ..Default::default() };
        raw_input(&mut app, &ctx, &mut first);
        app.doc_epoch += 1;
        app.engine.execute("track.new", &json!({"count": 1})).unwrap();
        let mut canceled = RawInput { events: vec![touch(1, TouchPhase::Cancel, pos2(180.0, 250.0))], ..Default::default() };
        raw_input(&mut app, &ctx, &mut canceled);
        assert_eq!(app.engine.session().tracks.len(), 1, "an old touch cannot restore a replaced document");
    }

    #[test]
    fn compact_resize_keeps_document_and_desktop_docks_unchanged() {
        let mut app = app();
        let ctx = egui::Context::default();
        let before = serde_json::to_value(app.engine.session()).unwrap();
        let dock = serde_json::to_value(&app.ui.docking).unwrap();
        for size in [vec2(390.0, 700.0), vec2(844.0, 320.0), vec2(1440.0, 960.0)] {
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), ..Default::default() };
            ctx.run_ui(raw, |ui| begin_frame(&mut app, ui)).textures_delta.clear();
        }
        assert!(!app.compact.active);
        assert_eq!(serde_json::to_value(app.engine.session()).unwrap(), before);
        assert_eq!(serde_json::to_value(&app.ui.docking).unwrap(), dock);
        assert!(!app.engine.is_dirty());
    }
}

pub(crate) fn desktop_choice(app: &mut SoundApp, ui: &mut egui::Ui) {
    app.compact.controls.clear();
    app.compact.body = Rect::NOTHING;
    egui::Panel::top("academy_audio_workspace").exact_size(44.0).show(ui, |ui| {
        let response = ui.menu_button("Workspace", |ui| preference_menu(app, ui)).response;
        app.compact.controls.insert("Workspace", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
    });
}

pub(crate) fn show(app: &mut SoundApp, ui: &mut egui::Ui) {
    app.compact.controls.clear();
    let tokens = crate::theme::Tokens::current();
    egui::Panel::top("academy_audio_compact_commands").exact_size(148.0).frame(egui::Frame::NONE.fill(tokens.toolbar_bg)).show(ui, |ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        ui.spacing_mut().interact_size.y = 44.0;
        ui.horizontal(|ui| {
            for (label, command) in [("Open", "session.open"), ("Save", "session.save"), ("Undo", "edit.undo"), ("Redo", "edit.redo")] {
                let enabled = match command {
                    "edit.undo" => app.engine.can_undo(),
                    "edit.redo" => app.engine.can_redo(),
                    _ => true,
                };
                let response = ui.add_enabled(enabled, egui::Button::new(label).min_size(vec2(54.0, 44.0)));
                app.compact.controls.insert(label, [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
                if response.clicked() {
                    menus::invoke_menu(app, command, "");
                }
            }
            let response = button(ui, "More", 54.0);
            egui::Popup::menu(&response).show(|ui| {
                preference_menu(app, ui);
                ui.separator();
                for (label, command) in [("Import audio", "file.import_audio"), ("Export audio", "file.bounce_mix"), ("New session", "session.new")] {
                    if button(ui, label, 150.0).clicked() {
                        menus::invoke_menu(app, command, "");
                        ui.close();
                    }
                }
            });
            app.compact.controls.insert("More", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
        });
        ui.horizontal(|ui| {
            let response = button(ui, if app.engine.transport.playing { "Pause" } else { "Play" }, 54.0);
            app.compact.controls.insert("Play", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
            if response.clicked() {
                let id = if app.engine.transport.playing { "transport.pause" } else { "transport.play" };
                let _ = app.run(id, json!({}));
            }
            let response = button(ui, "Stop", 48.0);
            app.compact.controls.insert("Stop", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
            if response.clicked() {
                let _ = app.run("transport.stop", json!({}));
            }
            let seconds = app.engine.transport.position as f64 / f64::from(app.engine.session().sample_rate.hz());
            ui.monospace(format!("{:02}:{:05.2}", (seconds / 60.0) as u64, seconds % 60.0));
            let response = ui.add_sized(vec2(46.0, 44.0), egui::Button::new("Edit").selected(!app.compact.navigate));
            app.compact.controls.insert("Edit", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
            if response.clicked() {
                app.compact.navigate = false;
            }
            let response = ui.add_sized(vec2(78.0, 44.0), egui::Button::new("Navigate").selected(app.compact.navigate));
            app.compact.controls.insert("Navigate", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
            if response.clicked() {
                app.compact.navigate = true;
            }
        });
        egui::ScrollArea::horizontal().id_salt("academy_audio_compact_tools").show(ui, |ui| {
            ui.horizontal(|ui| {
                let response = button(ui, "Tools", 54.0);
                egui::Popup::menu(&response).show(|ui| {
                    for (label, tool) in [("Select region", "selector"), ("Trim clip", "trim"), ("Move clip", "grabber"), ("Smart tool", "smart")] {
                        if button(ui, label, 150.0).clicked() {
                            let _ = app.run("edit.tool", json!({"tool": tool}));
                            app.compact.navigate = false;
                            ui.close();
                        }
                    }
                    for (label, id) in [("Trim to selection", "edit.trim_to_selection"), ("Separate at selection", "edit.separate"), ("Loop playback", "options.loop_playback")] {
                        if button(ui, label, 170.0).clicked() {
                            menus::invoke_menu(app, id, "");
                            ui.close();
                        }
                    }
                });
                app.compact.controls.insert("Tools", [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
                for (label, drawer) in [("Tracks", Drawer::Tracks), ("Clips", Drawer::Clips), ("Mixer", Drawer::Mixer)] {
                    let response = button(ui, label, 52.0);
                    app.compact.controls.insert(label, [response.rect.min.x, response.rect.min.y, response.rect.max.x, response.rect.max.y]);
                    if response.clicked() {
                        app.compact.drawer = if app.compact.drawer == drawer { Drawer::None } else { drawer };
                    }
                }
                for (label, command) in [("−", "view.zoom_out"), ("+", "view.zoom_in")] {
                    if button(ui, label, 44.0).clicked() {
                        let _ = app.run(command, json!({}));
                    }
                }
            });
        });
    });
    app.compact.body = ui.available_rect_before_wrap();
    if app.compact.drawer != Drawer::None {
        ui.horizontal(|ui| {
            if button(ui, "Back to tracks", 130.0).clicked() {
                app.compact.drawer = Drawer::None;
            }
        });
    }
    if ui.available_width() < 160.0 || ui.available_height() < 64.0 {
        ui.label("Close the keyboard to resume the workspace.");
        return;
    }
    // Top panels consume available space, but the original root's max_rect still includes
    // those panels. The session painter intentionally fills its entire max_rect. Give every
    // workspace/drawer its own bounded child so it cannot paint over the pinned controls.
    let body_rect = ui.available_rect_before_wrap();
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
    body.set_clip_rect(body_rect.intersect(ui.clip_rect()));
    app.compact.body = body_rect;
    match app.compact.drawer {
        Drawer::None => crate::edit_window::compact_area(app, &mut body),
        Drawer::Tracks => panels::tracks_and_groups(app, &mut body),
        Drawer::Clips => panels::clip_list(app, &mut body),
        Drawer::Mixer => crate::mix_window::content(app, &mut body),
    }
}
