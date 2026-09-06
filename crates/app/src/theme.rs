//! Every colour the page uses. A theme paints the chrome, the canvas, and — when it
//! brings a palette — the group colours too. Add an entry to [`THEMES`] and it shows up
//! in the settings dialog.
//!
//! The colours are written as CSS ever wrote them, `#rrggbb` and `rgba(…)`, and [`color`]
//! is the one place that turns a spec into something the painter takes. A theme is a
//! struct rather than a table of keys, so a colour that is missing from one theme is a
//! compile error instead of a hole in the window.

use brain_map_model::Group;
use eframe::egui::{Color32, Visuals};

pub struct Theme {
    pub key: &'static str,
    pub name: &'static str,
    // Chrome, which becomes egui's own palette.
    pub bg: &'static str,
    pub panel: &'static str,
    pub border: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub input_bg: &'static str,
    pub input_border: &'static str,
    pub btn_bg: &'static str,
    pub btn_hover: &'static str,
    // Prose, which the reader draws notes with.
    pub heading: &'static str,
    pub strong: &'static str,
    pub code: &'static str,
    // The graph, read by the renderer each frame.
    pub canvas_bg: &'static str,
    pub link: &'static str,
    pub link_lit: &'static str,
    pub link_dim: &'static str,
    pub label: &'static str,
    /// Themes without a palette keep the colours the vault itself decided.
    pub palette: Option<&'static [&'static str]>,
    pub router: Option<&'static str>,
}

pub const THEMES: &[Theme] = &[
    Theme {
        key: "midnight",
        name: "Midnight",
        bg: "#07090d",
        panel: "rgba(10,13,18,.85)",
        border: "#1f2937",
        text: "#e5e7eb",
        muted: "#9ca3af",
        accent: "#34d399",
        input_bg: "#0d1320",
        input_border: "#283548",
        btn_bg: "#111827",
        btn_hover: "#1c2a3f",
        heading: "#f1f5f9",
        strong: "#fde68a",
        code: "#f9a8d4",
        canvas_bg: "#07090d",
        link: "rgba(90,110,140,.28)",
        link_lit: "rgba(120,160,210,.7)",
        link_dim: "rgba(90,110,140,.05)",
        label: "rgba(229,231,235,.85)",
        palette: None,
        router: None,
    },
    Theme {
        key: "onedark",
        name: "One Dark",
        bg: "#282c34",
        panel: "rgba(33,37,43,.9)",
        border: "#3e4451",
        text: "#abb2bf",
        muted: "#5c6370",
        accent: "#98c379",
        input_bg: "#21252b",
        input_border: "#3e4451",
        btn_bg: "#2c313a",
        btn_hover: "#3e4451",
        heading: "#d7dae0",
        strong: "#e5c07b",
        code: "#e06c75",
        canvas_bg: "#282c34",
        link: "rgba(92,99,112,.4)",
        link_lit: "rgba(97,175,239,.75)",
        link_dim: "rgba(92,99,112,.08)",
        label: "rgba(171,178,191,.9)",
        palette: Some(&[
            "#61afef", "#e5c07b", "#c678dd", "#98c379", "#e06c75", "#56b6c2", "#d19a66", "#be5046",
            "#7f848e", "#abb2bf", "#528bff", "#c8ae9d",
        ]),
        router: Some("#98c379"),
    },
];

impl Theme {
    /// Every colour this theme names, so a theme that leaves one blank or misspells one
    /// fails a test rather than painting a hole in the window.
    #[cfg(test)]
    pub fn colours(&self) -> [(&'static str, &'static str); 18] {
        [
            ("bg", self.bg),
            ("panel", self.panel),
            ("border", self.border),
            ("text", self.text),
            ("muted", self.muted),
            ("accent", self.accent),
            ("input_bg", self.input_bg),
            ("input_border", self.input_border),
            ("btn_bg", self.btn_bg),
            ("btn_hover", self.btn_hover),
            ("heading", self.heading),
            ("strong", self.strong),
            ("code", self.code),
            ("canvas_bg", self.canvas_bg),
            ("link", self.link),
            ("link_lit", self.link_lit),
            ("link_dim", self.link_dim),
            ("label", self.label),
        ]
    }

    /// The chrome, as egui paints it. Only colours are set: the spacing and the shapes
    /// are egui's own, the way the platform's `<dialog>` used to be the browser's.
    pub fn visuals(&self) -> Visuals {
        let mut visuals = Visuals::dark();
        visuals.override_text_color = Some(color(self.text));
        visuals.panel_fill = color(self.bg);
        visuals.window_fill = color(self.panel);
        visuals.extreme_bg_color = color(self.input_bg);
        visuals.faint_bg_color = color(self.btn_bg);
        visuals.hyperlink_color = color(self.accent);
        visuals.window_stroke.color = color(self.border);
        visuals.selection.bg_fill = color(self.accent).gamma_multiply(0.35);
        visuals.selection.stroke.color = color(self.accent);
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = color(self.panel);
        widgets.noninteractive.bg_stroke.color = color(self.border);
        widgets.noninteractive.fg_stroke.color = color(self.muted);
        for widget in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            widget.bg_stroke.color = color(self.input_border);
            widget.fg_stroke.color = color(self.text);
        }
        widgets.inactive.bg_fill = color(self.btn_bg);
        widgets.inactive.weak_bg_fill = color(self.btn_bg);
        widgets.hovered.bg_fill = color(self.btn_hover);
        widgets.hovered.weak_bg_fill = color(self.btn_hover);
        widgets.active.bg_fill = color(self.btn_hover);
        widgets.active.weak_bg_fill = color(self.btn_hover);
        visuals
    }

    /// A group's colour, ready to paint with.
    pub fn group_colour(&self, groups: &[Group], key: &str) -> Color32 {
        color(&self.group_color(groups, key))
    }

    /// A group's colour under this theme: its own, unless the theme brought a palette.
    pub fn group_color(&self, groups: &[Group], key: &str) -> String {
        let Some(palette) = self.palette else {
            return groups
                .iter()
                .find(|g| g.key == key)
                .map_or_else(|| self.muted.to_string(), |g| g.color.clone());
        };
        match key {
            "router" => return self.router.unwrap_or(self.accent).to_string(),
            "external" => return self.muted.to_string(),
            _ => {}
        }
        let at = groups
            .iter()
            .filter(|g| g.key != "router" && g.key != "external")
            .position(|g| g.key == key)
            .unwrap_or(0);
        palette[at % palette.len()].to_string()
    }
}

/// A CSS colour as the themes write them: `#rrggbb`, `#rrggbbaa`, or `rgba(r,g,b,a)`.
/// Anything else is a typo in the table, and shows up as the magenta egui reserves for
/// exactly that rather than as a silent black.
pub fn color(spec: &str) -> Color32 {
    let spec = spec.trim();
    if let Some(hex) = spec.strip_prefix('#') {
        let channel = |at: usize| u8::from_str_radix(hex.get(at..at + 2).unwrap_or(""), 16).ok();
        let rgb = (channel(0), channel(2), channel(4));
        if let (Some(r), Some(g), Some(b)) = rgb {
            return match (hex.len(), channel(6)) {
                (8, Some(a)) => Color32::from_rgba_unmultiplied(r, g, b, a),
                (6, _) => Color32::from_rgb(r, g, b),
                _ => Color32::PLACEHOLDER,
            };
        }
    }
    if let Some(inner) = spec.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let mut parts = inner.split(',').map(str::trim);
        let mut channel = || parts.next().and_then(|p| p.parse::<u8>().ok());
        let rgb = (channel(), channel(), channel());
        // The alpha is a fraction, and `.28` is how a stylesheet writes 0.28.
        let alpha = parts.next().and_then(|a| {
            format!("0{a}")
                .parse::<f32>()
                .ok()
                .or_else(|| a.parse().ok())
        });
        if let ((Some(r), Some(g), Some(b)), Some(a)) = (rgb, alpha) {
            return Color32::from_rgba_unmultiplied(
                r,
                g,
                b,
                (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            );
        }
    }
    Color32::PLACEHOLDER
}

pub fn by_key(key: &str) -> Option<usize> {
    THEMES.iter().position(|t| t.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<Group> {
        ["router", "ideas", "daily", "external"]
            .iter()
            .map(|key| Group {
                key: (*key).to_string(),
                color: "#123456".into(),
                radius: 6.0,
                glow: 0.0,
                name: (*key).to_string(),
                pace: 60,
                pause: 450,
                major: true,
                cluster: false,
            })
            .collect()
    }

    #[test]
    fn every_theme_paints_every_colour_it_names() {
        for theme in THEMES {
            for (name, spec) in theme.colours() {
                assert!(!spec.is_empty(), "{} left {name} empty", theme.key);
                assert_ne!(
                    color(spec),
                    Color32::PLACEHOLDER,
                    "{} wrote {name} as {spec}",
                    theme.key
                );
            }
        }
    }

    #[test]
    fn colours_are_read_the_way_a_stylesheet_writes_them() {
        assert_eq!(color("#07090d"), Color32::from_rgb(7, 9, 13));
        assert_eq!(
            color("#0000ff80"),
            Color32::from_rgba_unmultiplied(0, 0, 255, 128)
        );
        assert_eq!(
            color("rgba(90,110,140,.28)"),
            Color32::from_rgba_unmultiplied(90, 110, 140, 71)
        );
        assert_eq!(
            color("rgba(1, 2, 3, 1)"),
            Color32::from_rgba_unmultiplied(1, 2, 3, 255)
        );
        assert_eq!(
            color("chartreuse"),
            Color32::PLACEHOLDER,
            "a name is not a spec"
        );
        assert_eq!(
            color("#abc"),
            Color32::PLACEHOLDER,
            "and neither is a short hex"
        );
    }

    #[test]
    fn a_theme_without_a_palette_keeps_the_vaults_own_colours() {
        let midnight = &THEMES[by_key("midnight").expect("midnight")];
        assert_eq!(midnight.group_color(&groups(), "ideas"), "#123456");
    }

    #[test]
    fn a_palette_repaints_the_groups_but_spares_the_structure() {
        let onedark = &THEMES[by_key("onedark").expect("onedark")];
        let groups = groups();
        assert_eq!(onedark.group_color(&groups, "router"), "#98c379");
        assert_eq!(onedark.group_color(&groups, "external"), onedark.muted);
        assert_eq!(
            onedark.group_color(&groups, "ideas"),
            "#61afef",
            "first in the palette"
        );
        assert_eq!(
            onedark.group_color(&groups, "daily"),
            "#e5c07b",
            "then the second"
        );
        assert_eq!(
            onedark.group_color(&groups, "nothing"),
            "#61afef",
            "unknown falls to first"
        );
    }
}
