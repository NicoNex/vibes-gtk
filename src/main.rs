//! Vibes — a chromatic instrument tuner.
//!
//! GTK4 / libadwaita port of the Android original: same editorial look (flowing wavy bands, a
//! scalloped note sticker, physics-y motion), rebuilt on Adwaita's own colours and widgets
//! instead of Material 3 Expressive.

mod audio;
mod config;
mod paint;
mod pitch;
mod settings;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use relm4::gtk::gio;
use relm4::gtk::pango;
use relm4::prelude::*;
use relm4::{adw, gtk};

use config::Config;
use paint::{Anim, Palette, Rgb};
use pitch::note_info;
use settings::Settings;

const IN_TUNE_CENTS: f32 = 5.0;

/// A Pango attribute list holding one absolute font size, in device pixels.
fn absolute_size(px: f64) -> pango::AttrList {
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrSize::new_size_absolute((px * pango::SCALE as f64) as i32));
    attrs
}

struct App {
    cfg: Config,
    freq: f32,
    error: Option<String>,
    anim: Rc<RefCell<Anim>>,
    engine: Option<audio::Engine>,
    settings: Controller<Settings>,
    window: adw::ApplicationWindow,
    /// The blob's current on-screen diameter, which the note typography is sized from.
    blob_px: f64,
}

#[derive(Debug)]
enum Msg {
    Freq(f32),
    OpenSettings,
    CfgChanged(Config),
    ThemeChanged,
    BlobResized(f64),
}

// ---------------------------------------------------------------------------------------------
// Derived state — one source of truth for what the pitch currently means.
// ---------------------------------------------------------------------------------------------

impl App {
    fn cents(&self) -> f32 {
        if self.freq > 0.0 {
            note_info(self.freq, self.cfg.a4).cents
        } else {
            0.0
        }
    }

    fn in_tune(&self) -> bool {
        self.freq > 0.0 && self.cents().abs() <= IN_TUNE_CENTS
    }

    fn state_color(&self) -> Rgb {
        let p = self.anim.borrow().palette;
        if self.freq <= 0.0 {
            p.muted
        } else if self.in_tune() {
            p.accent
        } else if self.cents() > 0.0 {
            p.warning
        } else {
            p.error
        }
    }

    /// The status pill: what it says, and the CSS class that colours it.
    fn chip(&self) -> (&'static str, &'static str) {
        if self.freq <= 0.0 {
            ("LISTENING", "idle")
        } else if self.in_tune() {
            ("IN TUNE", "tune")
        } else if self.cents() > 0.0 {
            ("SHARP  ↓", "sharp") // too high → tune down
        } else {
            ("FLAT  ↑", "flat") // too low → tune up
        }
    }

    fn note_text(&self) -> &'static str {
        if self.freq > 0.0 {
            self.cfg.note_name(note_info(self.freq, self.cfg.a4).pitch_class)
        } else {
            // Empty, not a dash: an em dash at this size reads as a redaction bar. The pill
            // already says LISTENING and the hint below says what to do.
            ""
        }
    }

    fn octave_text(&self) -> String {
        if self.freq > 0.0 {
            note_info(self.freq, self.cfg.a4).octave.to_string()
        } else {
            String::new()
        }
    }

    fn note_classes(&self, base: &'static str) -> [&'static str; 2] {
        let ink = if self.state_color().wants_light_text() { "on-dark" } else { "on-light" };
        [base, ink]
    }

    /// The note is sized from the blob it sits in, not from a fixed stack of CSS sizes: that is
    /// what lets the same layout hold on a phone. Longer names (solfège sharps like "Sol#")
    /// take a smaller share so they stay inside the scallops.
    fn note_px(&self) -> f64 {
        let share = match self.note_text().chars().count() {
            0..=2 => 0.46,
            3 => 0.36,
            _ => 0.28,
        };
        self.blob_px * share
    }

    fn note_attrs(&self) -> pango::AttrList {
        absolute_size(self.note_px())
    }

    fn octave_attrs(&self) -> pango::AttrList {
        absolute_size(self.note_px() * 0.32)
    }

    fn octave_margin(&self) -> i32 {
        (self.note_px() * 0.18) as i32
    }

    fn readout(&self) -> String {
        let cents = self.cents().round() as i32;
        format!(
            "{} Hz  ·  {}{} cents",
            self.freq.round() as i32,
            if cents >= 0 { "+" } else { "" },
            cents
        )
    }

    /// Push the current reading into the animation state the drawing areas read every frame.
    fn sync_anim(&self) {
        self.anim.borrow_mut().set_pitch(self.freq, self.cents(), self.in_tune());
    }
}

// ---------------------------------------------------------------------------------------------

/// Wires the drawing functions, the frame clock and the live theme watch onto the widgets —
/// all the plumbing `init` would otherwise have to carry.
fn setup_painting(
    widgets: &AppWidgets,
    anim: &Rc<RefCell<Anim>>,
    root: &adw::ApplicationWindow,
    sender: &ComponentSender<App>,
) {
    // --- painting -----------------------------------------------------------------------
    let a = anim.clone();
    widgets
        .waves
        .set_draw_func(move |_, cr, w, h| paint::draw_waves(cr, w as f64, h as f64, &a.borrow()));
    let a = anim.clone();
    widgets
        .blob
        .set_draw_func(move |_, cr, w, h| paint::draw_blob(cr, w as f64, h as f64, &a.borrow()));
    let a = anim.clone();
    widgets.arrows_up.set_draw_func(move |_, cr, w, h| {
        let an = a.borrow();
        paint::draw_chevrons(cr, w as f64, h as f64, true, an.up, &an);
    });
    let a = anim.clone();
    widgets.arrows_down.set_draw_func(move |_, cr, w, h| {
        let an = a.borrow();
        paint::draw_chevrons(cr, w as f64, h as f64, false, an.down, &an);
    });

    // The content floats over a headerbar that paints nothing, so it has to start below it.
    // AdwToolbarView publishes the bar's real height for exactly this.
    widgets
        .toolbar
        .bind_property("top-bar-height", &widgets.content, "margin-top")
        .sync_create()
        .build();

    // The note typography follows the blob's real size, so it fits on a phone and grows on
    // a desktop without a table of breakpoints.
    let s = sender.input_sender().clone();
    widgets.blob.connect_resize(move |_, w, h| {
        let _ = s.send(Msg::BlobResized((w.min(h) as f64).min(paint::MAX_BLOB_PX)));
    });

    // --- the frame clock ----------------------------------------------------------------
    // 60 fps of motion never touches the relm4 update loop: the tick advances the shared
    // animation state and asks the four areas to redraw.
    let a = anim.clone();
    let waves = widgets.waves.clone();
    let blob = widgets.blob.clone();
    let up = widgets.arrows_up.clone();
    let down = widgets.arrows_down.clone();
    let last = std::cell::Cell::new(0i64);
    let arrows_was = std::cell::Cell::new((0f32, 0f32));
    root.add_tick_callback(move |_, clock| {
        let now = clock.frame_time();
        let prev = last.replace(now);
        let dt = if prev == 0 { 0.0 } else { (now - prev) as f64 / 1_000_000.0 };
        // Clamp so a stalled frame (resize, wake from sleep) never jumps the motion.
        let arrows = {
            let mut anim = a.borrow_mut();
            anim.step(dt.min(0.05));
            (anim.up, anim.down)
        };
        waves.queue_draw();
        blob.queue_draw();
        // An empty chevron row has nothing to repaint. Redraw while it shows, plus the
        // one frame after it empties, so the last ghost is cleared.
        let was = arrows_was.replace(arrows);
        if arrows.0 > 0.004 || was.0 > 0.004 {
            up.queue_draw();
        }
        if arrows.1 > 0.004 || was.1 > 0.004 {
            down.queue_draw();
        }
        relm4::gtk::glib::ControlFlow::Continue
    });

    // --- follow the system theme live ---------------------------------------------------
    let style = adw::StyleManager::default();
    let s = sender.input_sender().clone();
    style.connect_dark_notify(move |_| {
        let _ = s.send(Msg::ThemeChanged);
    });
    let s = sender.input_sender().clone();
    style.connect_accent_color_notify(move |_| {
        let _ = s.send(Msg::ThemeChanged);
    });
    let s = sender.input_sender().clone();
    style.connect_high_contrast_notify(move |_| {
        let _ = s.send(Msg::ThemeChanged);
    });
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = ();
    type Input = Msg;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some("Vibes"),
            set_default_width: 460,
            set_default_height: 880,
            // GNOME Mobile's floor is 360×294. Everything below sizes itself from the space it
            // is given, so the same layout holds from a phone to a maximised desktop window.
            set_size_request: (360, 294),

            #[wrap(Some)]
            #[name = "toolbar"]
            set_content = &adw::ToolbarView {
                // The wave field runs edge to edge, under a headerbar that paints nothing.
                set_extend_content_to_top_edge: true,
                set_top_bar_style: adw::ToolbarStyle::Flat,

                add_top_bar = &adw::HeaderBar {
                    set_show_title: false,
                    add_css_class: "vibes-header",

                    pack_end = &gtk::MenuButton {
                        set_icon_name: "open-menu-symbolic",
                        set_tooltip_text: Some("Main Menu"),
                        set_primary: true,
                        set_menu_model: Some(&main_menu),
                        // Adwaita's style for controls laid over content: legible on any band.
                        add_css_class: "osd",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Overlay {
                    #[wrap(Some)]
                    #[name = "waves"]
                    set_child = &gtk::DrawingArea {
                        set_hexpand: true,
                        set_vexpand: true,
                    },

                    #[name = "content"]
                    add_overlay = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_start: 12,
                        set_margin_end: 12,
                        set_margin_bottom: 24,
                        // margin-top is bound to the headerbar's height in init: the waves run
                        // under the bar, the controls must not run into the status pill.
                        #[watch]
                        set_visible: model.error.is_none(),

                        gtk::Label {
                            set_halign: gtk::Align::Center,
                            #[watch]
                            set_label: model.chip().0,
                            #[watch]
                            set_css_classes: &["chip", model.chip().1],
                        },

                        // Arrows above the note point UP when the pitch is flat (raise it).
                        #[name = "arrows_up"]
                        gtk::DrawingArea {
                            set_size_request: (-1, 30),
                            set_vexpand: true,
                        },

                        gtk::Overlay {
                            set_vexpand: true,

                            #[wrap(Some)]
                            #[name = "blob"]
                            set_child = &gtk::DrawingArea {
                                set_size_request: (120, 120),
                            },

                            add_overlay = &gtk::Box {
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                                set_spacing: 4,

                                // The octave rides to the right of the note, which would push
                                // the note itself off-centre in the blob. This invisible twin
                                // balances it exactly, with no measuring and no guesswork.
                                gtk::Label {
                                    set_opacity: 0.0,
                                    set_can_target: false,
                                    #[watch]
                                    set_label: &model.octave_text(),
                                    #[watch]
                                    set_attributes: Some(&model.octave_attrs()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: model.note_text(),
                                    #[watch]
                                    set_css_classes: &model.note_classes("note"),
                                    #[watch]
                                    set_attributes: Some(&model.note_attrs()),
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &model.octave_text(),
                                    #[watch]
                                    set_css_classes: &model.note_classes("octave"),
                                    #[watch]
                                    set_attributes: Some(&model.octave_attrs()),
                                    #[watch]
                                    set_margin_top: model.octave_margin(),
                                },
                            },
                        },

                        // Arrows below point DOWN when sharp (lower it).
                        #[name = "arrows_down"]
                        gtk::DrawingArea {
                            set_size_request: (-1, 30),
                            set_vexpand: true,
                        },

                        gtk::Box {
                            set_halign: gtk::Align::Center,
                            set_height_request: 48,

                            gtk::Label {
                                add_css_class: "readout",
                                set_valign: gtk::Align::Center,
                                #[watch]
                                set_visible: model.freq > 0.0,
                                #[watch]
                                set_label: &model.readout(),
                            },
                            gtk::Label {
                                add_css_class: "hint",
                                set_label: "Play a note",
                                set_valign: gtk::Align::Center,
                                #[watch]
                                set_visible: model.freq <= 0.0,
                            },
                        },
                    },

                    add_overlay = &gtk::Box {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_margin_all: 24,
                        add_css_class: "card",
                        #[watch]
                        set_visible: model.error.is_some(),

                        adw::StatusPage {
                            set_icon_name: Some("microphone-disabled-symbolic"),
                            set_title: "Microphone unavailable",
                            #[watch]
                            set_description: model.error.as_deref(),
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let cfg = Config::load();

        let settings =
            Settings::builder().launch(cfg).forward(sender.input_sender(), Msg::CfgChanged);

        // VIBES_DEMO_HZ pins the reading to a fixed frequency and leaves the microphone shut.
        // It exists so the documentation screenshots can be taken without an instrument, and
        // without playing a sound into whatever room the machine is sitting in.
        let demo = std::env::var("VIBES_DEMO_HZ")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            // "inf" parses fine and then spins forever in the octave-shift loop.
            .filter(|hz| hz.is_finite() && *hz >= 0.0)
            // 0 means "demo mode, but heard nothing" — the idle screen, with the mic still shut.
            .map(|hz| if hz > 0.0 { hz } else { -1.0 });

        // Otherwise the detector runs on its own thread and posts every window back into the
        // relm4 loop.
        let (freq, engine, error) = match demo {
            Some(hz) => (hz, None, None),
            None => {
                let freq_sender = sender.input_sender().clone();
                match audio::start(move |freq| {
                    let _ = freq_sender.send(Msg::Freq(freq));
                }) {
                    Ok(engine) => {
                        engine.set_hold_seconds(cfg.sustain);
                        (-1.0, Some(engine), None)
                    }
                    Err(e) => (-1.0, None, Some(e)),
                }
            }
        };

        let model = App {
            cfg,
            freq,
            error,
            anim: Rc::new(RefCell::new(Anim::new(Palette::current()))),
            engine,
            settings,
            window: root.clone(),
            blob_px: paint::MAX_BLOB_PX,
        };

        let main_menu = gio::Menu::new();
        main_menu.append(Some("_Preferences"), Some("win.preferences"));
        main_menu.append(Some("_About Vibes"), Some("win.about"));

        let widgets = view_output!();

        let s = sender.input_sender().clone();
        let preferences = gio::SimpleAction::new("preferences", None);
        preferences.connect_activate(move |_, _| {
            let _ = s.send(Msg::OpenSettings);
        });
        root.add_action(&preferences);
        let about = gio::SimpleAction::new("about", None);
        let win = root.clone();
        about.connect_activate(move |_, _| {
            adw::AboutDialog::builder()
                .application_name("Vibes")
                .application_icon("com.niconex.Vibes")
                .developer_name("Nicolò Santamaria")
                .version(env!("CARGO_PKG_VERSION"))
                .license_type(gtk::License::Gpl30)
                .build()
                .present(Some(&win));
        });
        root.add_action(&about);
        let app = relm4::main_application();
        app.set_accels_for_action("win.preferences", &["<Control>comma"]);
        // One window, so closing it is quitting.
        app.set_accels_for_action("window.close", &["<Control>w", "<Control>q"]);

        setup_painting(&widgets, &model.anim, &root, &sender);
        // VIBES_DEMO_SETTINGS opens the settings window on start-up, so it can be photographed
        // without driving the pointer.
        if demo.is_some() && std::env::var("VIBES_DEMO_SETTINGS").is_ok() {
            sender.input(Msg::OpenSettings);
        }

        model.sync_anim();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Msg::Freq(freq) => self.freq = freq,
            Msg::OpenSettings => self.settings.widget().present(Some(&self.window)),
            Msg::CfgChanged(cfg) => {
                self.cfg = cfg;
                if let Some(engine) = &self.engine {
                    engine.set_hold_seconds(cfg.sustain);
                }
            }
            Msg::BlobResized(px) => self.blob_px = px,
            Msg::ThemeChanged => {
                let palette = Palette::current();
                // Scoped: sync_anim borrows the same RefCell the moment this arm ends.
                {
                    let mut anim = self.anim.borrow_mut();
                    anim.palette = palette;
                    anim.blob = palette.muted;
                }
            }
        }
        self.sync_anim();
    }
}

fn main() {
    // The ID is also the icon name and the desktop file's basename: Wayland matches the window
    // to its .desktop entry by app ID, and that is what puts the icon in the shell.
    let app = RelmApp::new("com.niconex.Vibes");
    gtk::Window::set_default_icon_name("com.niconex.Vibes");
    relm4::set_global_css(include_str!("style.css"));
    app.run::<App>(());
}
