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
use relm4::prelude::*;
use relm4::{adw, gtk};

use config::Config;
use paint::{Anim, Palette, Rgb};
use pitch::note_info;
use settings::Settings;

const IN_TUNE_CENTS: f32 = 5.0;

struct App {
    cfg: Config,
    freq: f32,
    error: Option<String>,
    anim: Rc<RefCell<Anim>>,
    engine: Option<audio::Engine>,
    settings: Controller<Settings>,
    window: adw::ApplicationWindow,
}

#[derive(Debug)]
enum Msg {
    Freq(f32),
    OpenSettings,
    CfgChanged(Config),
    ThemeChanged,
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
        match () {
            _ if self.freq <= 0.0 => p.muted,
            _ if self.in_tune() => p.accent,
            _ if self.cents() > 0.0 => p.warning,
            _ => p.error,
        }
    }

    fn chip_label(&self) -> &'static str {
        match () {
            _ if self.freq <= 0.0 => "LISTENING",
            _ if self.in_tune() => "IN TUNE",
            _ if self.cents() > 0.0 => "SHARP  ↓", // too high → tune down
            _ => "FLAT  ↑",                        // too low  → tune up
        }
    }

    fn chip_classes(&self) -> Vec<&'static str> {
        let state = match () {
            _ if self.freq <= 0.0 => "idle",
            _ if self.in_tune() => "tune",
            _ if self.cents() > 0.0 => "sharp",
            _ => "flat",
        };
        vec!["chip", state]
    }

    fn note_text(&self) -> String {
        if self.freq > 0.0 {
            self.cfg
                .note_name(note_info(self.freq, self.cfg.a4).pitch_class)
                .to_string()
        } else {
            "—".to_string()
        }
    }

    fn octave_text(&self) -> String {
        if self.freq > 0.0 {
            note_info(self.freq, self.cfg.a4).octave.to_string()
        } else {
            String::new()
        }
    }

    /// Longer names (solfège sharps like "Sol#") shrink so they stay inside the blob.
    fn len_class(&self) -> &'static str {
        match self.note_text().chars().count() {
            0..=2 => "len-1",
            3 => "len-3",
            _ => "len-4",
        }
    }

    fn note_classes(&self, base: &'static str) -> Vec<&'static str> {
        let ink = if self.state_color().wants_light_text() { "on-dark" } else { "on-light" };
        vec![base, self.len_class(), ink]
    }

    fn octave_margin(&self) -> i32 {
        match self.len_class() {
            "len-1" => 24,
            "len-3" => 19,
            _ => 15,
        }
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
        self.anim
            .borrow_mut()
            .set_pitch(self.freq, self.cents(), self.in_tune());
    }
}

// ---------------------------------------------------------------------------------------------

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

            #[wrap(Some)]
            set_content = &adw::ToolbarView {
                // The wave field runs edge to edge, under a headerbar that paints nothing.
                set_extend_content_to_top_edge: true,
                set_top_bar_style: adw::ToolbarStyle::Flat,

                add_top_bar = &adw::HeaderBar {
                    set_show_title: false,
                    add_css_class: "vibes-header",

                    pack_end = &gtk::Button {
                        set_icon_name: "emblem-system-symbolic",
                        set_tooltip_text: Some("Settings"),
                        add_css_class: "circular",
                        connect_clicked => Msg::OpenSettings,
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

                    add_overlay = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        #[watch]
                        set_visible: model.error.is_none(),

                        gtk::Label {
                            set_halign: gtk::Align::Center,
                            #[watch]
                            set_label: model.chip_label(),
                            #[watch]
                            set_css_classes: &model.chip_classes(),
                        },

                        // Arrows above the note point UP when the pitch is flat (raise it).
                        #[name = "arrows_up"]
                        gtk::DrawingArea {
                            set_content_width: 300,
                            set_content_height: 76,
                        },

                        gtk::Overlay {
                            #[wrap(Some)]
                            #[name = "blob"]
                            set_child = &gtk::DrawingArea {
                                set_content_width: 312,
                                set_content_height: 312,
                            },

                            add_overlay = &gtk::Box {
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                                set_spacing: 4,

                                gtk::Label {
                                    #[watch]
                                    set_label: &model.note_text(),
                                    #[watch]
                                    set_css_classes: &model.note_classes("note"),
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Start,
                                    #[watch]
                                    set_label: &model.octave_text(),
                                    #[watch]
                                    set_css_classes: &model.note_classes("octave"),
                                    #[watch]
                                    set_margin_top: model.octave_margin(),
                                },
                            },
                        },

                        // Arrows below point DOWN when sharp (lower it).
                        #[name = "arrows_down"]
                        gtk::DrawingArea {
                            set_content_width: 300,
                            set_content_height: 76,
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
                            adw::Spinner {
                                set_size_request: (28, 28),
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
        let anim = Rc::new(RefCell::new(Anim::new(Palette::current())));

        let settings = Settings::builder()
            .launch(cfg)
            .forward(sender.input_sender(), Msg::CfgChanged);

        // The detector runs on its own thread and posts every window back into the relm4 loop.
        let freq_sender = sender.input_sender().clone();
        let (engine, error) = match audio::start(move |f| {
            let _ = freq_sender.send(Msg::Freq(f));
        }) {
            Ok(engine) => {
                engine.set_hold_seconds(cfg.sustain);
                (Some(engine), None)
            }
            Err(e) => (None, Some(e)),
        };

        let model = App {
            cfg,
            freq: -1.0,
            error,
            anim: anim.clone(),
            engine,
            settings,
            window: root.clone(),
        };

        let widgets = view_output!();

        // --- painting -------------------------------------------------------------------
        {
            let a = anim.clone();
            widgets
                .waves
                .set_draw_func(move |_, cr, w, h| paint::draw_waves(cr, w as f64, h as f64, &a.borrow()));
        }
        {
            let a = anim.clone();
            widgets
                .blob
                .set_draw_func(move |_, cr, w, h| paint::draw_blob(cr, w as f64, h as f64, &a.borrow()));
        }
        {
            let a = anim.clone();
            widgets.arrows_up.set_draw_func(move |_, cr, w, h| {
                let an = a.borrow();
                paint::draw_chevrons(cr, w as f64, h as f64, true, an.up, &an);
            });
        }
        {
            let a = anim.clone();
            widgets.arrows_down.set_draw_func(move |_, cr, w, h| {
                let an = a.borrow();
                paint::draw_chevrons(cr, w as f64, h as f64, false, an.down, &an);
            });
        }

        // --- the frame clock ------------------------------------------------------------
        // 60 fps of motion never touches the relm4 update loop: the tick advances the shared
        // animation state and asks the four areas to redraw.
        {
            let a = anim.clone();
            let areas = [
                widgets.waves.clone(),
                widgets.blob.clone(),
                widgets.arrows_up.clone(),
                widgets.arrows_down.clone(),
            ];
            let last = std::cell::Cell::new(0i64);
            root.add_tick_callback(move |_, clock| {
                let now = clock.frame_time();
                let prev = last.replace(now);
                let dt = if prev == 0 { 0.0 } else { (now - prev) as f64 / 1_000_000.0 };
                // Clamp so a stalled frame (resize, wake from sleep) never jumps the motion.
                a.borrow_mut().step(dt.min(0.05));
                for area in &areas {
                    area.queue_draw();
                }
                relm4::gtk::glib::ControlFlow::Continue
            });
        }

        // --- follow the system theme live -----------------------------------------------
        {
            let style = adw::StyleManager::default();
            let s = sender.input_sender().clone();
            style.connect_dark_notify(move |_| {
                let _ = s.send(Msg::ThemeChanged);
            });
            let s = sender.input_sender().clone();
            style.connect_accent_color_notify(move |_| {
                let _ = s.send(Msg::ThemeChanged);
            });
        }

        model.sync_anim();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Msg::Freq(f) => self.freq = f,
            Msg::OpenSettings => {
                let win = self.settings.widget();
                win.set_transient_for(Some(&self.window));
                win.present();
            }
            Msg::CfgChanged(cfg) => {
                self.cfg = cfg;
                if let Some(engine) = &self.engine {
                    engine.set_hold_seconds(cfg.sustain);
                }
            }
            Msg::ThemeChanged => {
                let palette = Palette::current();
                let mut anim = self.anim.borrow_mut();
                anim.palette = palette;
                anim.blob = palette.muted;
            }
        }
        self.sync_anim();
    }
}

fn main() {
    let app = RelmApp::new("com.niconex.vibes");
    relm4::set_global_css(include_str!("style.css"));
    app.run::<App>(());
}
