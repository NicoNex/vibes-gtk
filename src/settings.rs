//! The settings sheet — an `AdwPreferencesDialog`, which is adaptive for free: a centred sheet
//! on a wide window, a full-height page on a narrow one.

use relm4::prelude::*;
use relm4::{adw, gtk};

use adw::prelude::*;

use crate::config::{Config, A4_RANGE, SUSTAIN_DEFAULT, SUSTAIN_RANGE};

pub struct Settings {
    cfg: Config,
}

#[derive(Debug)]
pub enum SettingsMsg {
    A4(f64),
    Sustain(f64),
    Solfege(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for Settings {
    type Init = Config;
    type Input = SettingsMsg;
    type Output = Config;

    view! {
        // A real top-level window, so it gets the system decorations and can be moved and
        // closed like any other — rather than a sheet locked inside the tuner.
        adw::Window {
            set_title: Some("Settings"),
            set_default_width: 520,
            set_default_height: 620,
            set_hide_on_close: true,

            #[wrap(Some)]
            set_content = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                set_content = &adw::PreferencesPage {
                    add = &adw::PreferencesGroup {
                        set_title: "Reference pitch (A4)",
                        #[wrap(Some)]
                        set_header_suffix = &gtk::Label {
                            add_css_class: "bignum",
                            #[watch]
                            set_label: &format!("{} Hz", model.cfg.a4.round() as i32),
                        },

                        set_description: Some("What the tuner calls concert A."),

                        adw::PreferencesRow {
                            set_activatable: false,
                            set_focusable: false,

                            #[wrap(Some)]
                            set_child = &gtk::Scale {
                                set_margin_all: 12,
                                set_hexpand: true,
                                set_draw_value: false,
                                set_round_digits: 0,
                                set_adjustment: &gtk::Adjustment::new(
                                    model.cfg.a4 as f64, A4_RANGE.0 as f64, A4_RANGE.1 as f64, 1.0, 5.0, 0.0,
                                ),
                                add_mark: (440.0, gtk::PositionType::Bottom, None),
                                connect_value_changed[sender] => move |s| {
                                    sender.input(SettingsMsg::A4(s.value()));
                                },
                            },
                        },
                    },

                    add = &adw::PreferencesGroup {
                        set_title: "Sustain",
                        set_description: Some("How long a note is kept after the string fades."),
                        #[wrap(Some)]
                        set_header_suffix = &gtk::Box {
                            set_spacing: 10,
                            set_valign: gtk::Align::Center,

                            gtk::Label {
                                add_css_class: "bignum",
                                #[watch]
                                set_label: &format!("{:.1} s", model.cfg.sustain),
                            },
                            gtk::Label {
                                add_css_class: "badge",
                                set_label: "DEFAULT",
                                set_valign: gtk::Align::Center,
                                #[watch]
                                set_visible: (model.cfg.sustain - SUSTAIN_DEFAULT).abs() < 0.001,
                            },
                        },

                        adw::PreferencesRow {
                            set_activatable: false,
                            set_focusable: false,

                            #[wrap(Some)]
                            set_child = &gtk::Scale {
                                set_margin_all: 12,
                                set_hexpand: true,
                                set_draw_value: false,
                                set_round_digits: 1,
                                set_adjustment: &gtk::Adjustment::new(
                                    model.cfg.sustain as f64, SUSTAIN_RANGE.0 as f64, SUSTAIN_RANGE.1 as f64,
                                    0.1, 0.5, 0.0,
                                ),
                                add_mark: (SUSTAIN_DEFAULT as f64, gtk::PositionType::Bottom, None),
                                // Written back so the magnetic detent below actually moves the knob.
                                #[watch]
                                set_value: model.cfg.sustain as f64,
                                connect_value_changed[sender] => move |s| {
                                    sender.input(SettingsMsg::Sustain(s.value()));
                                },
                            },
                        },
                    },

                    add = &adw::PreferencesGroup {
                        set_title: "Note names",

                        adw::PreferencesRow {
                            set_activatable: false,
                            set_focusable: false,

                            #[wrap(Some)]
                            set_child = &adw::ToggleGroup {
                                set_margin_all: 12,
                                set_hexpand: true,
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
            },
        }
    }

    fn init(
        cfg: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Settings { cfg };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::A4(v) => self.cfg.a4 = (v as f32).round().clamp(A4_RANGE.0, A4_RANGE.1),
            SettingsMsg::Sustain(v) => {
                // Magnetic detent on the default — the slider pulls into 1.2 s as you pass it.
                let raw = v as f32;
                let snapped = if (raw - SUSTAIN_DEFAULT).abs() < 0.06 { SUSTAIN_DEFAULT } else { raw };
                self.cfg.sustain = snapped.clamp(SUSTAIN_RANGE.0, SUSTAIN_RANGE.1);
            }
            SettingsMsg::Solfege(v) => self.cfg.solfege = v,
        }
        self.cfg.save();
        let _ = sender.output(self.cfg);
    }
}
