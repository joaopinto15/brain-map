//! One keystroke, one decision, carried out here.
//!
//! What a key means is [`crate::keys::binding`], which is tested without a window. This
//! only turns egui's events into that question and acts on the answer.

use super::bar::Bar;
use crate::app::Session;
use crate::keys::{binding, key_name, Action};
use eframe::egui::{self, Context, Id, Key};

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
        }
    }
}
