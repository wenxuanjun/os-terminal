use crate::color::Rgb;
use spin::Lazy;

pub const DEFAULT_PALETTE_INDEX: usize = 0;

pub struct Palette {
    pub foreground: Rgb,
    pub background: Rgb,
    pub ansi_colors: [Rgb; 16],
}

impl Palette {
    fn build(pair: (&str, &str), ansi_colors: [&str; 16]) -> Self {
        Self {
            foreground: Self::hex_to_rgb(pair.0),
            background: Self::hex_to_rgb(pair.1),
            ansi_colors: ansi_colors.map(Self::hex_to_rgb),
        }
    }

    fn hex_to_rgb(hex: &str) -> Rgb {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    }
}

pub static PALETTE_DATA: Lazy<[Palette; 8]> = Lazy::new(|| {
    [
        Palette::build(
            ("#f5f5f5", "#151515"),
            [
                "#151515", "#ac4142", "#90a959", "#f4bf75", "#6a9fb5", "#aa759f", "#75b5aa",
                "#d0d0d0", "#505050", "#ac4142", "#90a959", "#f4bf75", "#6a9fb5", "#aa759f",
                "#75b5aa", "#f5f5f5",
            ],
        ),
        Palette::build(
            ("#839496", "#002b36"),
            [
                "#002b36", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                "#eee8d5", "#073642", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                "#93a1a1", "#fdf6e3",
            ],
        ),
        Palette::build(
            ("#ffffff", "#300924"),
            [
                "#2e3436", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a",
                "#d3d7cf", "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8",
                "#34e2e2", "#eeeeec",
            ],
        ),
        Palette::build(
            ("#f8f8f2", "#121212"),
            [
                "#181d1e", "#f92672", "#a6e22e", "#fd971f", "#66d9ef", "#9e6ffe", "#5e7175",
                "#cccccc", "#505354", "#ff669d", "#beed5f", "#e6db74", "#66d9ef", "#9e6ffe",
                "#a3babf", "#f8f8f2",
            ],
        ),
        Palette::build(
            ("#00bb00", "#001100"),
            [
                "#001100", "#007700", "#00bb00", "#007700", "#009900", "#00bb00", "#005500",
                "#00bb00", "#007700", "#007700", "#00bb00", "#007700", "#009900", "#00bb00",
                "#005500", "#00ff00",
            ],
        ),
        Palette::build(
            ("#979db4", "#202746"),
            [
                "#202746", "#c94922", "#ac9739", "#c08b30", "#3d8fd1", "#6679cc", "#22a2c9",
                "#979db4", "#6b7394", "#c94922", "#ac9739", "#c08b30", "#3d8fd1", "#6679cc",
                "#22a2c9", "#f5f7ff",
            ],
        ),
        Palette::build(
            ("#657b83", "#fdf6e3"),
            [
                "#002b36", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                "#eee8d5", "#073642", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                "#93a1a1", "#fdf6e3",
            ],
        ),
        Palette::build(
            ("#26232a", "#efecf4"),
            [
                "#19171c", "#be4678", "#2a9292", "#a06e3b", "#576ddb", "#955ae7", "#398bc6",
                "#8b8792", "#585260", "#c9648e", "#34b2b2", "#bc8249", "#788ae2", "#ac7eed",
                "#599ecf", "#efecf4",
            ],
        ),
    ]
});
