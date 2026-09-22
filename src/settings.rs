//! The preferences — an `AdwPreferencesDialog`, which is adaptive for free: a floating sheet on a
//! wide window, a bottom sheet on a narrow one.

use relm4::prelude::*;
use relm4::{adw, gtk};

use adw::prelude::*;

use crate::config::{Config, A4_RANGE, SUSTAIN_DEFAULT, SUSTAIN_RANGE};

const A4_DEFAULT: f32 = 440.0;
use crate::paint::{self, Anim, Palette};

/// The pitch standards people actually tune to: baroque, 432, ISO 16, and the two orchestral ones.
const PRESETS: [f32; 5] = [415.0, 432.0, 440.0, 442.0, 443.0];

pub struct Settings {
    cfg: Config,
}

impl Settings {
    fn preset(&self) -> u32 {
        PRESETS
            .iter()
            .position(|p| *p == self.cfg.a4)
            .map_or(gtk::INVALID_LIST_POSITION, |i| i as u32)
    }

    fn a4_value(&self) -> f64 {
        self.cfg.a4 as f64
    }

    fn sustain_value(&self) -> f64 {
        self.cfg.sustain as f64
    }

    fn a4_text(&self) -> String {
        format!("{} Hz", self.cfg.a4.round() as i32)
    }

    fn sustain_text(&self) -> String {
        format!("{:.1} s", self.cfg.sustain)
    }

    fn a4_default(&self) -> bool {
        self.cfg.a4 == A4_DEFAULT
    }

    fn sustain_default(&self) -> bool {
        (self.cfg.sustain - SUSTAIN_DEFAULT).abs() < 0.001
    }

    /// The octave rides small and high beside the note, as it does in the tuner itself.
    fn preview_note(&self) -> String {
        format!("{}<span size=\"45%\" rise=\"16pt\">4</span>", self.cfg.note_name(9))
    }
}

#[derive(Debug)]
pub enum SettingsMsg {
    A4(f64),
    Preset(u32),
    Sustain(f64),
    Solfege(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for Settings {
    type Init = Config;
    type Input = SettingsMsg;
    type Output = Config;

    view! {
        adw::PreferencesDialog {
            set_title: "Preferences",
            set_search_enabled: false,

            add = &adw::PreferencesPage {
                // A live preview in the tuner's own sticker: what the chosen names and reference
                // pitch will look like before the dialog is closed.
                add = &adw::PreferencesGroup {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Center,
                        set_spacing: 6,

                        gtk::Overlay {
                            #[wrap(Some)]
                            #[name = "sticker"]
                            set_child = &gtk::DrawingArea {
                                set_content_width: 120,
                                set_content_height: 120,
                            },
                            add_overlay = &gtk::Label {
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                                set_css_classes: &["note", "preview-note", sticker_ink],
                                #[watch]
                                set_markup: &model.preview_note(),
                            },
                        },
                        gtk::Label {
                            add_css_class: "dim-label",
                            set_label: "Concert A",
                        },
                    },
                },

                add = &adw::PreferencesGroup {
                    set_title: "Tuning",

                    adw::PreferencesRow {
                        set_activatable: false,
                        set_focusable: false,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_top: 12,
                            set_margin_bottom: 10,
                            set_margin_start: 12,
                            set_margin_end: 12,
                            set_spacing: 2,

                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "Reference Pitch (A4)",
                            },
                            gtk::Box {
                                set_spacing: 10,
                                gtk::Label {
                                    add_css_class: "setting-value",
                                    #[watch]
                                    set_label: &model.a4_text(),
                                },
                                gtk::Label {
                                    add_css_class: "badge",
                                    set_label: "DEFAULT",
                                    set_valign: gtk::Align::Center,
                                    #[watch]
                                    set_visible: model.a4_default(),
                                },
                            },
                            #[name = "a4"]
                            gtk::Scale {
                                set_hexpand: true,
                                set_draw_value: false,
                                set_round_digits: 0,
                                set_adjustment: &gtk::Adjustment::new(model.cfg.a4 as f64, A4_RANGE.0 as f64, A4_RANGE.1 as f64, 1.0, 5.0, 0.0),
                                #[watch]
                                set_value: model.a4_value(),
                                #[watch]
                                update_property: &[gtk::accessible::Property::ValueText(&model.a4_text())],
                                connect_value_changed[sender] => move |s| {
                                    sender.input(SettingsMsg::A4(s.value()));
                                },
                            },
                            gtk::Label {
                                add_css_class: "dim-label",
                                add_css_class: "caption",
                                set_halign: gtk::Align::Start,
                                set_wrap: true,
                                set_xalign: 0.0,
                                set_label: "What the tuner calls concert A, in whole hertz.",
                            },
                        },
                    },

                    adw::ActionRow {
                        set_title: "Standard",

                        add_suffix = &adw::ToggleGroup {
                            set_valign: gtk::Align::Center,
                            add: adw::Toggle::builder().label("415").tooltip("Baroque").build(),
                            add: adw::Toggle::builder().label("432").tooltip("Verdi tuning").build(),
                            add: adw::Toggle::builder().label("440").tooltip("Modern concert pitch").build(),
                            add: adw::Toggle::builder().label("442").tooltip("Orchestral").build(),
                            add: adw::Toggle::builder().label("443").tooltip("Orchestral, central Europe").build(),
                            #[watch]
                            set_active: model.preset(),
                            connect_active_notify[sender] => move |g| {
                                sender.input(SettingsMsg::Preset(g.active()));
                            },
                        },
                    },

                    adw::PreferencesRow {
                        set_activatable: false,
                        set_focusable: false,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_top: 12,
                            set_margin_bottom: 10,
                            set_margin_start: 12,
                            set_margin_end: 12,
                            set_spacing: 2,

                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "Sustain",
                            },
                            gtk::Box {
                                set_spacing: 10,
                                gtk::Label {
                                    add_css_class: "setting-value",
                                    #[watch]
                                    set_label: &model.sustain_text(),
                                },
                                gtk::Label {
                                    add_css_class: "badge",
                                    set_label: "DEFAULT",
                                    set_valign: gtk::Align::Center,
                                    #[watch]
                                    set_visible: model.sustain_default(),
                                },
                            },
                            #[name = "sustain"]
                            gtk::Scale {
                                set_hexpand: true,
                                set_draw_value: false,
                                set_round_digits: 1,
                                set_adjustment: &gtk::Adjustment::new(model.cfg.sustain as f64, SUSTAIN_RANGE.0 as f64, SUSTAIN_RANGE.1 as f64, 0.1, 0.5, 0.0),
                                #[watch]
                                set_value: model.sustain_value(),
                                #[watch]
                                update_property: &[gtk::accessible::Property::ValueText(&model.sustain_text())],
                                connect_value_changed[sender] => move |s| {
                                    sender.input(SettingsMsg::Sustain(s.value()));
                                },
                            },
                            gtk::Label {
                                add_css_class: "dim-label",
                                add_css_class: "caption",
                                set_halign: gtk::Align::Start,
                                set_wrap: true,
                                set_xalign: 0.0,
                                set_label: "How long a note is kept after the string fades.",
                            },
                        },
                    },
                },

                add = &adw::PreferencesGroup {
                    set_title: "Display",

                    adw::ActionRow {
                        set_title: "Note Names",

                        add_suffix = &adw::ToggleGroup {
                            set_valign: gtk::Align::Center,
                            add: adw::Toggle::builder().label("A B C").build(),
                            add: adw::Toggle::builder().label("Do Re Mi").build(),
                            set_active: model.cfg.solfege as u32,
                            connect_active_notify[sender] => move |g| {
                                sender.input(SettingsMsg::Solfege(g.active() == 1));
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        cfg: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Settings { cfg };
        let sticker_ink =
            if Palette::current().accent.wants_light_text() { "on-dark" } else { "on-light" };
        let widgets = view_output!();

        // One tick per step, as on the Android sliders: the quantisation is visible, and the
        // default gets a labelled mark of its own.
        for hz in A4_RANGE.0 as i32..=A4_RANGE.1 as i32 {
            let label = (hz as f32 == A4_DEFAULT).then_some("440");
            widgets.a4.add_mark(hz as f64, gtk::PositionType::Bottom, label);
        }
        for tenth in (SUSTAIN_RANGE.0 * 10.0) as i32..=(SUSTAIN_RANGE.1 * 10.0) as i32 {
            let v = tenth as f64 / 10.0;
            let label = ((v as f32 - SUSTAIN_DEFAULT).abs() < 0.001).then_some("1.2");
            widgets.sustain.add_mark(v, gtk::PositionType::Bottom, label);
        }

        // The in-tune sticker, still: the accent colour, no ring, no sway.
        widgets.sticker.set_draw_func(|_, cr, w, h| {
            let palette = Palette::current();
            let mut anim = Anim::new(palette);
            anim.blob = palette.accent;
            anim.wobble_amp = 0.0;
            paint::draw_blob(cr, w as f64, h as f64, &anim);
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        let before = self.cfg;
        match msg {
            SettingsMsg::A4(v) => self.cfg.a4 = (v as f32).round(),
            SettingsMsg::Preset(i) => match PRESETS.get(i as usize) {
                Some(hz) => self.cfg.a4 = *hz,
                // Deselected because the pitch moved off every preset: nothing to change.
                None => return,
            },
            SettingsMsg::Sustain(v) => {
                // 0.1 s steps, with a magnetic detent that pulls the knob into the default.
                let stepped = (v as f32 * 10.0).round() / 10.0;
                self.cfg.sustain = if (v as f32 - SUSTAIN_DEFAULT).abs() < 0.06 {
                    SUSTAIN_DEFAULT
                } else {
                    stepped
                };
            }
            SettingsMsg::Solfege(v) => self.cfg.solfege = v,
        }
        // A slider fires on every pixel it is dragged across, but its value moves in whole
        // steps: the tap marks the step the setting landed on, not the drag.
        if self.cfg != before {
            crate::haptics::tap();
        }
        // Quantised values move in whole steps, so this writes once per step, not per pixel —
        // and nothing is lost if the app quits with the dialog still open.
        self.cfg.save();
        let _ = sender.output(self.cfg);
    }
}
