//! The graph's own region: everything the pointer does to it.
//!
//! Nothing here knows the camera's arithmetic. A wheel, a drag and a click are turned
//! into the [`crate::state::Engine`] methods that mean them, and the engine does the maths
//! it has always done.

use crate::app::Session;
use eframe::egui::{self, Context, Id, Sense};

#[derive(Default)]
pub struct Viewport;

impl Viewport {
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        // Central, so it is whatever the panels left over: a drag inside the explorer is
        // the explorer's, never the camera's.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let region = ui.interact(ui.max_rect(), Id::new("graph"), Sense::click_and_drag());
                let engine = &mut session.engine;

                if region.hovered() {
                    let notches = ui.input(|i| i.smooth_scroll_delta.y) as f64;
                    if let (true, Some(at)) = (notches != 0.0, region.hover_pos()) {
                        engine.zoom(at, notches);
                    }
                }
                if let (true, Some(at)) = (region.drag_started(), region.interact_pointer_pos()) {
                    engine.grab(at);
                }
                if region.dragged() {
                    match (engine.dragging_node(), region.interact_pointer_pos()) {
                        (true, Some(at)) => engine.drag_to(at),
                        (true, None) => {}
                        (false, _) => {
                            let by = region.drag_delta();
                            engine.pan_by(by.x, by.y);
                        }
                    }
                }
                if region.drag_stopped() {
                    engine.let_go();
                }
                // A click is a drag that went nowhere, which is egui's own distinction rather
                // than one counted here.
                if let (true, Some(at)) = (region.clicked(), region.interact_pointer_pos()) {
                    engine.click(at);
                }
                engine.hover(region.hover_pos());
            });
    }
}
