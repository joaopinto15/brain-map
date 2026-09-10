//! The window, as one object per job.
//!
//! [`Session`] is what is open: the vault, the graph built from it, the emoji, and the
//! [`Source`] that answers for anything on disk. [`Chrome`] is what is drawn over it, and
//! every panel owns the state only it cares about. They are separate fields so that a
//! panel can be handed the whole session to act on without borrowing itself.

use crate::chrome::Chrome;
use crate::emoji::Icons;
use crate::markdown::{Block, Target};
use crate::state::{Engine, Ui};
use brain_map_model::{Graph, Source};
use eframe::egui::{self, Context};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How often the vault is asked whether it has changed.
const POLL: Duration = Duration::from_secs(2);

pub struct App {
    session: Session,
    chrome: Chrome,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        source: Arc<dyn Source>,
        vault: Option<PathBuf>,
    ) -> App {
        App {
            session: Session::new(cc, source, vault),
            chrome: Chrome::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        self.session.begin_frame(ctx);
        self.chrome.show(ctx, &mut self.session);
        self.session.end_frame(ctx);
    }
}

/// The open vault and everything derived from it.
pub struct Session {
    pub engine: Engine,
    /// The emoji pictures, loaded once out of the desktop's font.
    pub icons: Icons,
    source: Arc<dyn Source>,
    vault: Option<PathBuf>,
    /// A vault being opened on a worker thread, and what went wrong with the last one.
    /// The dialog and the clone take as long as they take, and a frame that waits on one
    /// is a window the compositor calls not responding.
    job: Option<Receiver<Result<PathBuf, String>>>,
    error: String,
    /// The open note, parsed the once when the reader moves rather than every frame.
    note: Option<(usize, Vec<Block>)>,
    /// The vault named on the command line, waiting for the first frame: egui has no
    /// fonts until then, and a name cannot be measured before there is something to
    /// measure it with.
    pending: Option<PathBuf>,
    /// The vault's fingerprint as it was last seen. `None` until the first poll, which is
    /// only a baseline: reloading on it would never stop.
    mark: Option<u64>,
    polled: Instant,
}

impl Session {
    fn new(
        cc: &eframe::CreationContext<'_>,
        source: Arc<dyn Source>,
        vault: Option<PathBuf>,
    ) -> Session {
        let ui = Ui::new(vault.is_some());
        cc.egui_ctx.set_visuals(ui.theme().visuals());
        // An empty graph measures nothing, so this one is safe to build before there are
        // fonts. The vault it was asked for arrives on the first frame.
        Session {
            engine: Engine::new(Graph::default(), ui, &cc.egui_ctx),
            icons: Icons::load(),
            source,
            vault: None,
            note: None,
            pending: vault,
            mark: None,
            polled: Instant::now(),
            job: None,
            error: String::new(),
        }
    }

    /// Everything before the chrome: the window's size, the vault, one step of the
    /// simulation, and the graph painted on the layer the panels sit over.
    fn begin_frame(&mut self, ctx: &Context) {
        // The window's size is known before anything is loaded into it: a vault opened
        // against a viewport that is still zero would clamp the explorer to its minimum
        // and never grow back.
        let screen = ctx.screen_rect();
        self.engine
            .resize(screen.width() as f64, screen.height() as f64);
        self.finish_open(ctx);
        if let Some(vault) = self.pending.take() {
            self.load(ctx, vault, true);
        }
        self.engine
            .frame(ctx.input(|i| i.stable_dt) as f64 * 1000.0);
        self.poll(ctx);
        let painter = ctx.layer_painter(egui::LayerId::background());
        self.engine.draw(&painter, screen, &mut self.icons);
    }

    fn end_frame(&mut self, ctx: &Context) {
        self.sync_note();
        // The simulation is always moving, and so is the growth clock.
        ctx.request_repaint();
    }

    pub fn vault(&self) -> Option<&Path> {
        self.vault.as_deref()
    }

    pub fn note(&self) -> Option<&[Block]> {
        self.note.as_ref().map(|(_, blocks)| blocks.as_slice())
    }

    pub fn reading_id(&self) -> Option<String> {
        self.engine
            .ui()
            .reading()
            .map(|at| self.engine.node_id(at).to_string())
    }

    /// The ⟳ button: the same scan the watcher does, asked for on purpose.
    pub fn rescan(&mut self, ctx: &Context) {
        if let Some(vault) = self.vault.clone() {
            self.load(ctx, vault, true);
        }
    }

    /// A path someone typed or picked, opened off the frame thread.
    pub fn open_vault(&mut self, typed: &str) {
        let typed = typed.to_string();
        self.spawn(move |source| source.open_vault(&typed));
    }

    /// The desktop's folder dialog, and the vault it names. One job, because the dialog
    /// stays open for as long as someone is browsing in it.
    pub fn browse_vault(&mut self) {
        self.spawn(|source| {
            let chosen = source
                .choose_folder()?
                .ok_or_else(|| "no folder chosen".to_string())?;
            source.open_vault(&chosen)
        });
    }

    /// True while a vault is being opened, so the picker says so and asks for no second one.
    pub fn opening(&self) -> bool {
        self.job.is_some()
    }

    /// Why the last open did not happen, for whoever asked for it to show.
    pub fn open_error(&self) -> &str {
        &self.error
    }

    fn spawn(
        &mut self,
        work: impl FnOnce(&dyn Source) -> Result<PathBuf, String> + Send + 'static,
    ) {
        if self.job.is_some() {
            return;
        }
        self.error.clear();
        let source = Arc::clone(&self.source);
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        std::thread::spawn(move || tx.send(work(source.as_ref())));
    }

    /// The worker's answer, taken on the frame that finds it. The window repaints every
    /// frame anyway, so there is nothing to wake.
    fn finish_open(&mut self, ctx: &Context) {
        let Some(job) = &self.job else {
            return;
        };
        match job.try_recv() {
            Err(TryRecvError::Empty) => return,
            Ok(Ok(dir)) => {
                // Remembered before the load, so the Ui the load builds reads the fresh list.
                self.engine
                    .ui_mut()
                    .remember_vault(&dir.display().to_string());
                self.load(ctx, dir, true);
                self.engine.ui_mut().close_picker();
            }
            Ok(Err(said)) => self.error = said,
            Err(TryRecvError::Disconnected) => self.error = "the vault could not be opened".into(),
        }
        self.job = None;
    }

    pub fn drives(&self) -> Vec<String> {
        self.source.drives()
    }

    pub fn edit_open_note(&self) {
        if let (Some(vault), Some(id)) = (self.vault.as_deref(), self.reading_id()) {
            self.source.edit(vault, &id);
        }
    }

    /// A link inside a note. A note resolves the way the graph resolved it into an edge,
    /// and one that is not there does nothing rather than going somewhere wrong.
    pub fn follow(&mut self, target: Target) {
        match target {
            // ponytail: the desktop's opener, which is already what runs $EDITOR's terminal.
            Target::Url(url) => {
                let _ = std::process::Command::new("xdg-open").arg(url).spawn();
            }
            Target::Note { path, relative } => {
                let from = self.reading_id();
                if let Some(node) = self.engine.resolve(&path, from.as_deref(), relative) {
                    self.engine.go_to(node);
                }
            }
        }
    }

    /// A vault chosen, or one that changed under us. `growth` is false for a reload the
    /// watcher asked for: the note being read comes back and the graph does not replay.
    fn load(&mut self, ctx: &Context, vault: PathBuf, growth: bool) {
        let open = self.reading_id();
        let graph = self.source.scan(&vault);
        let mut ui = Ui::new(true);
        ui.inherit(self.engine.ui());
        let budget = match growth {
            true => ui.budget(),
            false => 0,
        };
        let (w, h) = (
            ctx.screen_rect().width() as f64,
            ctx.screen_rect().height() as f64,
        );
        self.engine = Engine::new(graph, ui, ctx);
        self.engine.resize(w, h);
        self.engine.schedule(budget);
        self.engine.restart();
        self.vault = Some(vault);
        self.mark = None;
        self.note = None;
        // The note that was open comes back, if the change did not take it away.
        if let Some(node) = open
            .filter(|_| !growth)
            .and_then(|id| self.engine.find(&id))
        {
            self.engine.open(Some(node));
        }
    }

    /// The vault is on disk and can change under the window. There is no notifier in the
    /// standard library and a stat walk is not worth a dependency, so it is asked.
    fn poll(&mut self, ctx: &Context) {
        if self.polled.elapsed() < POLL {
            return;
        }
        self.polled = Instant::now();
        let Some(vault) = self.vault.clone() else {
            return;
        };
        let now = self.source.fingerprint(&vault);
        let moved = crate::filter::changed(self.mark.as_ref(), &now);
        self.mark = Some(now);
        if moved {
            self.load(ctx, vault, false);
        }
    }

    /// Reading a note is a disk read, so it happens when the reader moves to another one
    /// rather than on every frame.
    fn sync_note(&mut self) {
        let reading = self.engine.ui().reading();
        if self.note.as_ref().map(|(at, _)| *at) == reading {
            return;
        }
        self.note = match (reading, self.vault.as_deref()) {
            (Some(at), Some(vault)) => {
                let id = self.engine.node_id(at).to_string();
                let text = self.source.read_note(vault, &id).unwrap_or_default();
                Some((at, crate::markdown::render(&text)))
            }
            _ => None,
        };
    }
}
