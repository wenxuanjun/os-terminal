use crate::color::Rgb888;

pub const DEFAULT_PALETTE_INDEX: usize = 0;

#[repr(C)]
pub struct Palette {
    pub foreground: Rgb888,
    pub background: Rgb888,
    pub ansi_colors: [Rgb888; 16],
}

pub const PALETTE_DATA: [Palette; 8] = [
    Palette {
        foreground: (0xf5, 0xf5, 0xf5),
        background: (0x15, 0x15, 0x15),
        ansi_colors: [
            (0x15, 0x15, 0x15),
            (0xac, 0x41, 0x42),
            (0x90, 0xa9, 0x59),
            (0xf4, 0xbf, 0x75),
            (0x6a, 0x9f, 0xb5),
            (0xaa, 0x75, 0x9f),
            (0x75, 0xb5, 0xaa),
            (0xd0, 0xd0, 0xd0),
            (0x50, 0x50, 0x50),
            (0xac, 0x41, 0x42),
            (0x90, 0xa9, 0x59),
            (0xf4, 0xbf, 0x75),
            (0x6a, 0x9f, 0xb5),
            (0xaa, 0x75, 0x9f),
            (0x75, 0xb5, 0xaa),
            (0xf5, 0xf5, 0xf5),
        ],
    },
    Palette {
        foreground: (0xff, 0xff, 0xff),
        background: (0x00, 0x00, 0x00),
        ansi_colors: [
            (0x00, 0x00, 0x00),
            (0xcd, 0x00, 0x00),
            (0x00, 0xcd, 0x00),
            (0xcd, 0xcd, 0x00),
            (0x00, 0x00, 0xee),
            (0xcd, 0x00, 0xcd),
            (0x00, 0xcd, 0xcd),
            (0xe5, 0xe5, 0xe5),
            (0x7f, 0x7f, 0x7f),
            (0xff, 0x00, 0x00),
            (0x00, 0xff, 0x00),
            (0xff, 0xff, 0x00),
            (0x5c, 0x5c, 0xff),
            (0xff, 0x00, 0xff),
            (0x00, 0xff, 0xff),
            (0xff, 0xff, 0xff),
        ],
    },
    Palette {
        foreground: (0x93, 0xa1, 0xa1),
        background: (0x00, 0x2b, 0x36),
        ansi_colors: [
            (0x00, 0x2b, 0x36),
            (0xdc, 0x32, 0x2f),
            (0x85, 0x99, 0x00),
            (0xb5, 0x89, 0x00),
            (0x26, 0x8b, 0xd2),
            (0x6c, 0x71, 0xc4),
            (0x2a, 0xa1, 0x98),
            (0x93, 0xa1, 0xa1),
            (0x65, 0x7b, 0x83),
            (0xdc, 0x32, 0x2f),
            (0x85, 0x99, 0x00),
            (0xb5, 0x89, 0x00),
            (0x26, 0x8b, 0xd2),
            (0x6c, 0x71, 0xc4),
            (0x2a, 0xa1, 0x98),
            (0xfd, 0xf6, 0xe3),
        ],
    },
    Palette {
        foreground: (0xff, 0xff, 0xff),
        background: (0x30, 0x09, 0x24),
        ansi_colors: [
            (0x2e, 0x34, 0x36),
            (0xcc, 0x00, 0x00),
            (0x4e, 0x9a, 0x06),
            (0xc4, 0xa0, 0x00),
            (0x34, 0x65, 0xa4),
            (0x75, 0x50, 0x7b),
            (0x06, 0x98, 0x9a),
            (0xd3, 0xd7, 0xcf),
            (0x55, 0x57, 0x53),
            (0xef, 0x29, 0x29),
            (0x8a, 0xe2, 0x34),
            (0xfc, 0xe9, 0x4f),
            (0x72, 0x9f, 0xcf),
            (0xad, 0x7f, 0xa8),
            (0x34, 0xe2, 0xe2),
            (0xee, 0xee, 0xec),
        ],
    },
    Palette {
        foreground: (0x00, 0xbb, 0x00),
        background: (0x00, 0x11, 0x00),
        ansi_colors: [
            (0x00, 0x11, 0x00),
            (0x00, 0x77, 0x00),
            (0x00, 0xbb, 0x00),
            (0x00, 0x77, 0x00),
            (0x00, 0x99, 0x00),
            (0x00, 0xbb, 0x00),
            (0x00, 0x55, 0x00),
            (0x00, 0xbb, 0x00),
            (0x00, 0x77, 0x00),
            (0x00, 0x77, 0x00),
            (0x00, 0xbb, 0x00),
            (0x00, 0x77, 0x00),
            (0x00, 0x99, 0x00),
            (0x00, 0xbb, 0x00),
            (0x00, 0x55, 0x00),
            (0x00, 0xff, 0x00),
        ],
    },
    Palette {
        foreground: (0x97, 0x9d, 0xb4),
        background: (0x20, 0x27, 0x46),
        ansi_colors: [
            (0x20, 0x27, 0x46),
            (0xc9, 0x49, 0x22),
            (0xac, 0x97, 0x39),
            (0xc0, 0x8b, 0x30),
            (0x3d, 0x8f, 0xd1),
            (0x66, 0x79, 0xcc),
            (0x22, 0xa2, 0xc9),
            (0x97, 0x9d, 0xb4),
            (0x6b, 0x73, 0x94),
            (0xc9, 0x49, 0x22),
            (0xac, 0x97, 0x39),
            (0xc0, 0x8b, 0x30),
            (0x3d, 0x8f, 0xd1),
            (0x66, 0x79, 0xcc),
            (0x22, 0xa2, 0xc9),
            (0xf5, 0xf7, 0xff),
        ],
    },
    Palette {
        foreground: (0x58, 0x6e, 0x75),
        background: (0xfd, 0xf6, 0xe3),
        ansi_colors: [
            (0xfd, 0xf6, 0xe3),
            (0xdc, 0x32, 0x2f),
            (0x85, 0x99, 0x00),
            (0xb5, 0x89, 0x00),
            (0x26, 0x8b, 0xd2),
            (0x6c, 0x71, 0xc4),
            (0x2a, 0xa1, 0x98),
            (0x58, 0x6e, 0x75),
            (0x83, 0x94, 0x96),
            (0xdc, 0x32, 0x2f),
            (0x85, 0x99, 0x00),
            (0xb5, 0x89, 0x00),
            (0x26, 0x8b, 0xd2),
            (0x6c, 0x71, 0xc4),
            (0x2a, 0xa1, 0x98),
            (0x00, 0x2b, 0x36),
        ],
    },
    Palette {
        foreground: (0x58, 0x52, 0x60),
        background: (0xef, 0xec, 0xf4),
        ansi_colors: [
            (0xef, 0xec, 0xf4),
            (0xbe, 0x46, 0x78),
            (0x2a, 0x92, 0x92),
            (0xa0, 0x6e, 0x3b),
            (0x57, 0x6d, 0xdb),
            (0x95, 0x5a, 0xe7),
            (0x39, 0x8b, 0xc6),
            (0x58, 0x52, 0x60),
            (0x7e, 0x78, 0x87),
            (0xbe, 0x46, 0x78),
            (0x2a, 0x92, 0x92),
            (0xa0, 0x6e, 0x3b),
            (0x57, 0x6d, 0xdb),
            (0x95, 0x5a, 0xe7),
            (0x39, 0x8b, 0xc6),
            (0x19, 0x17, 0x1c),
        ],
    },
];
