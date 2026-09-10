//! One keystroke, one decision, carried out here.
//!
//! What a key means is [`crate::keys::binding`], which is tested without a window. This
//! only turns egui's events into that question and acts on the answer.

use super::bar::Bar;
use crate::app::Session;
use crate::keys::{binding, key_name, Action};
use eframe::egui::{self, Context, Id, Key};

/// How far `j` moves the reader, and how far `g` and `G` throw it. A scroll area clamps
/// what it is given, so the whole note is one keystroke either way.
const LINE: f32 = 28.0;
const PAGE: f32 = 1.0e6;

#[derive(Default)]
pub struct Keymap {
    /// Space, waiting for the key it leads.
    leader: bool,
}

impl Keymap {
    pub fn handle(&mut self, ctx: &Context, session: &mut Session) {
        // A keystroke inside a field is typing: `r`, `f` and `/` are all letters someone
        // will type, so none of them may reach the keymap.
        let typing = ctx.memory(|m| m.focused().is_some());
        let pressed: Vec<(Key, bool)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, modifiers.shift)),
                    _ => None,
                })
                .collect()
        });

        for (key, shift) in pressed {
            // The picker takes Esc even while the cursor is in its own field, but only
            // once there is a vault to go back to.
            if key == Key::Escape && session.engine.ui().picker_open() && session.vault().is_some()
            {
                session.engine.ui_mut().close_picker();
                continue;
            }
            let Some(name) = key_name(key, shift) else {
                continue;
            };
            let leader = std::mem::take(&mut self.leader);
            let Some(action) = binding(&name, leader, typing) else {
                continue;
            };
            self.act(ctx, session, action);
        }
    }

    fn act(&mut self, ctx: &Context, session: &mut Session, action: Action) {
        let engine = &mut session.engine;
        match action {
            Action::Arm => self.leader = true,
            Action::ToggleExplorer => {
                let hidden = engine.ui().explorer_hidden();
                engine.hide_panel(!hidden);
            }
            Action::FocusSearch => {
                ctx.memory_mut(|m| m.request_focus(Id::new(Bar::SEARCH)));
                // The field is focused in the same frame, so the `/` that focused it
                // would otherwise be the first thing typed into it.
                ctx.input_mut(|i| {
                    i.events
                        .retain(|e| !matches!(e, egui::Event::Text(t) if t == "/"))
                });
            }
            Action::Release => engine.release(),
            Action::Replay => engine.restart(),
            Action::ToggleFull => engine.ui_mut().toggle_full(),
            Action::NextMatch => engine.step(1),
            Action::PrevMatch => engine.step(-1),
            Action::Pan(dx, dy) => engine.pan_step(dx, dy),
            // The panel shows one of two things, so the same key moves whichever it is:
            // the note when one is open, the tree when none is.
            Action::Down => match engine.ui().reading().is_some() {
                true => engine.ui_mut().scroll_by(-LINE),
                false => engine.move_cursor(1),
            },
            Action::Up => match engine.ui().reading().is_some() {
                true => engine.ui_mut().scroll_by(LINE),
                false => engine.move_cursor(-1),
            },
            Action::Out => match engine.ui().reading().is_some() {
                // Out of the note is back to the tree it was chosen from.
                true => engine.close_note(),
                false => engine.cursor_out(),
            },
            // The tree is not on screen while a note is: reading it again is a key away,
            // and Esc or `h` is what puts the tree back.
            Action::Into => {
                if engine.ui().reading().is_none() {
                    engine.cursor_into();
                }
            }
            Action::Top => match engine.ui().reading().is_some() {
                true => engine.ui_mut().scroll_by(PAGE),
                false => engine.cursor_edge(false),
            },
            Action::Bottom => match engine.ui().reading().is_some() {
                true => engine.ui_mut().scroll_by(-PAGE),
                false => engine.cursor_edge(true),
            },
        }
    }
}
