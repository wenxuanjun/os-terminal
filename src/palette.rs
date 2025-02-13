use crate::color::Rgb;

pub const DEFAULT_PALETTE_INDEX: usize = 0;

pub struct Palette {
    pub foreground: Rgb,
    pub background: Rgb,
    pub ansi_colors: [Rgb; 16],
}

impl Palette {
    const fn build(pair: (u32, u32), colors: [u32; 16]) -> Self {
        Self {
            foreground: Self::hex_to_rgb(pair.0),
            background: Self::hex_to_rgb(pair.1),
            ansi_colors: {
                let mut ansi_colors = [(0, 0, 0); 16];
                let mut i = 0;
                while i < 16 {
                    ansi_colors[i] = Self::hex_to_rgb(colors[i]);
                    i += 1;
                }
                ansi_colors
            },
        }
    }

    const fn hex_to_rgb(hex: u32) -> Rgb {
        ((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
    }
}

pub const PALETTE: [Palette; 8] = [
    Palette::build(
        (0xf5f5f5, 0x151515),
        [
            0x151515, 0xac4142, 0x90a959, 0xf4bf75, 0x6a9fb5, 0xaa759f, 0x75b5aa, 0xd0d0d0,
            0x505050, 0xac4142, 0x90a959, 0xf4bf75, 0x6a9fb5, 0xaa759f, 0x75b5aa, 0xf5f5f5,
        ],
    ),
    Palette::build(
        (0x839496, 0x002b36),
        [
            0x002b36, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5,
            0x073642, 0xcb4b16, 0x586e75, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3,
        ],
    ),
    Palette::build(
        (0xffffff, 0x300924),
        [
            0x2e3436, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf,
            0x555753, 0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
        ],
    ),
    Palette::build(
        (0xf8f8f2, 0x121212),
        [
            0x181d1e, 0xf92672, 0xa6e22e, 0xfd971f, 0x66d9ef, 0x9e6ffe, 0x5e7175, 0xcccccc,
            0x505354, 0xff669d, 0xbeed5f, 0xe6db74, 0x66d9ef, 0x9e6ffe, 0xa3babf, 0xf8f8f2,
        ],
    ),
    Palette::build(
        (0x00bb00, 0x001100),
        [
            0x001100, 0x007700, 0x00bb00, 0x007700, 0x009900, 0x00bb00, 0x005500, 0x00bb00,
            0x007700, 0x007700, 0x00bb00, 0x007700, 0x009900, 0x00bb00, 0x005500, 0x00ff00,
        ],
    ),
    Palette::build(
        (0x979db4, 0x202746),
        [
            0x202746, 0xc94922, 0xac9739, 0xc08b30, 0x3d8fd1, 0x6679cc, 0x22a2c9, 0x979db4,
            0x6b7394, 0xc94922, 0xac9739, 0xc08b30, 0x3d8fd1, 0x6679cc, 0x22a2c9, 0xf5f7ff,
        ],
    ),
    Palette::build(
        (0x657b83, 0xfdf6e3),
        [
            0x002b36, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5,
            0x073642, 0xcb4b16, 0x586e75, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3,
        ],
    ),
    Palette::build(
        (0x26232a, 0xefecf4),
        [
            0x19171c, 0xbe4678, 0x2a9292, 0xa06e3b, 0x576ddb, 0x955ae7, 0x398bc6, 0x8b8792,
            0x585260, 0xc9648e, 0x34b2b2, 0xbc8249, 0x788ae2, 0xac7eed, 0x599ecf, 0xefecf4,
        ],
    ),
];
