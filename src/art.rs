//! The looks: every sprite, and the house baked into one background image.
//! Nothing here knows about the story; skip this file to learn how the game works.

use crate::house::{self, PORTALS, ROOMS};

pub const CANVAS_W: usize = 400;
pub const CANVAS_H: usize = 192;

/// Furniture that does nothing: room, x centre, height of its bottom above the floor, pixels.
pub const DECOR: &[(&str, i32, i32, &[&str])] = &[
    ("attic", 185, 0, TRUNK),
    ("attic", 282, 0, BOXES),
    ("attic", 205, 21, ROUND_WINDOW),
    ("bedroom", 124, 0, NIGHTSTAND),
    ("bedroom", 108, 16, WINDOW),
    ("bedroom", 78, 21, PICTURE),
    ("bedroom", 176, 0, WARDROBE),
    ("bathroom", 292, 0, TUB),
    ("bathroom", 250, 19, MIRROR),
    ("bathroom", 220, 11, TOWEL),
    ("bathroom", 292, 19, WINDOW),
    ("study", 80, 0, BOOKSHELF),
    ("study", 150, 0, ARMCHAIR),
    ("study", 176, 0, FLOOR_LAMP),
    ("study", 128, 21, PICTURE),
    ("study", 150, 25, CLOCK),
    ("kitchen", 262, 19, WINDOW),
    ("kitchen", 298, 26, CUPBOARD),
    ("kitchen", 212, 20, JARS),
    ("cellar", 180, 0, BARREL),
    ("cellar", 204, 0, COAL),
    ("cellar", 268, 19, JARS),
    ("cellar", 320, 0, BOILER),
];

struct Canvas(Vec<u8>);

impl Canvas {
    fn set(&mut self, x: i32, y: i32, c: u32) {
        if (0..CANVAS_W as i32).contains(&x) && (0..CANVAS_H as i32).contains(&y) {
            let i = (y as usize * CANVAS_W + x as usize) * 4;
            self.0[i..i + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
        }
    }

    fn fill(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: u32) {
        self.paint(x0, y0, w, h, |_, _| c);
    }

    fn paint(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: impl Fn(i32, i32) -> u32) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.set(x, y, c(x, y));
            }
        }
    }

    /// Draws `rows` centred on x with the bottom row just above `bottom`.
    fn blit(&mut self, x: i32, bottom: i32, rows: &[&str]) {
        let w = rows.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
        let top = bottom - rows.len() as i32;
        for (j, row) in rows.iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                if let Some(c) = color(ch) {
                    self.set(x - w / 2 + i as i32, top + j as i32, c);
                }
            }
        }
    }
}

fn wallpaper(id: &str, x: i32, y: i32, floor: i32) -> u32 {
    let from_floor = floor - y;
    match id {
        "bedroom" => if x % 8 < 2 { 0xa87f68 } else { 0xbc9478 },
        "bathroom" if from_floor < 20 => if x % 6 == 0 || y % 6 == 0 { 0x3a7f7c } else { 0x5ab0aa },
        "bathroom" => 0x8ccac0,
        "study" if from_floor < 12 => if x % 14 == 0 || from_floor == 11 { 0x33241a } else { 0x4a3424 },
        "study" => if (x + y * 2) % 10 == 0 { 0x628a5c } else { 0x4c7048 },
        "kitchen" if from_floor < 12 => if (x / 4 + y / 4) % 2 == 0 { 0x9dbb9a } else { 0x7fa384 },
        "kitchen" => if x % 6 == 0 && y % 6 == 0 { 0xbc9c4c } else { 0xd8b866 },
        "cellar" => {
            if y % 8 == 0 || (x + (y / 8) % 2 * 6) % 12 == 0 { 0x3a342e } else { 0x544c44 }
        }
        _ => if x % 9 == 0 { 0x4e3a2c } else { 0x6a5240 },
    }
}

/// The whole house, RGBA, sky transparent.
pub fn background() -> Vec<u8> {
    let mut c = Canvas(vec![0; CANVAS_W * CANVAS_H * 4]);
    c.blit(50, 30, CLOUD);
    c.blit(366, 42, CLOUD);

    // Ground: soil with stones, grass, and a few flowers.
    c.paint(0, 145, 400, 47, |x, y| match (x * 7 + y * 13) % 83 {
        0 => 0x7a6450,
        1..=3 => 0x3e2c1e,
        _ => 0x4e3826,
    });
    c.paint(0, 142, 400, 3, |x, y| if (x + y) % 5 == 0 { 0x3f7a3a } else { 0x5c9e48 });
    for (x, col) in [(8, 'r'), (16, 'y'), (44, 'P'), (352, 'y'), (362, 'r'), (384, 'P')] {
        c.set(x, 141, color(col).unwrap_or(0));
        c.set(x, 142, 0x2f5a30);
    }

    // A tree and a bush.
    c.paint(24, 96, 8, 46, |x, _| if x % 3 == 0 { 0x4a2e1a } else { 0x6a4426 });
    for (cx, cy, r) in [(28_i32, 84_i32, 20_i32), (14, 94, 12), (42, 94, 12), (372, 134, 10), (384, 136, 8)] {
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                if (x - cx).pow(2) + (y - cy).pow(2) <= r * r {
                    let shade = match (x * 5 + y * 3) % 7 {
                        0 => 0x2f6a34,
                        1 => 0x6aa850,
                        _ => 0x468a42,
                    };
                    c.set(x, y, shade);
                }
            }
        }
    }

    // Chimney, then the roof over it, then the brick front.
    c.paint(270, 18, 13, 22, |x, y| brick(x, y));
    c.fill(268, 16, 17, 3, 0x5a3024);
    for y in 12..=63 {
        let half = (26 + (y - 12) * 124 / 50) / 2 * 2;
        c.paint(200 - half, y, half * 2, 1, |x, y| {
            let shift = (y / 4) % 2 * 3;
            if y % 4 == 3 { 0x74301f } else if (x + shift) % 6 == 0 { 0x8a3c28 } else { 0xb4543a }
        });
    }
    c.fill(174, 12, 52, 2, 0x5a2a1c);
    c.paint(58, 64, 284, 82, |x, y| brick(x, y));
    c.paint(152, 145, 190, 40, |x, y| if y % 6 == 0 || x % 11 == 0 { 0x38322c } else { 0x5a544c });

    // Attic: follows the slope of the roof.
    for y in 26..62 {
        let half = (26 + (y - 12) * 124 / 50) / 2 * 2 - 12;
        c.paint(200 - half, y, half * 2, 1, |x, y| wallpaper("attic", x, y, 62));
    }
    c.fill(62, 62, 276, 2, 0x5a3a22);

    for r in ROOMS.iter().filter(|r| r.id != "attic") {
        let (x0, w, floor) = (r.x.0 as i32 - 4, (r.x.1 - r.x.0) as i32 + 8, r.floor as i32);
        c.paint(x0, floor - 38, w, 38, |x, y| wallpaper(r.id, x, y, floor));
        if r.id != "kitchen" && r.id != "bathroom" {
            c.fill(x0, floor - 2, w, 2, 0x3a2a20);
        }
        c.paint(x0, floor, w, 3, |x, _| if x % 12 == 0 { 0x3e281a } else { 0x6e4a2c });
    }

    for p in PORTALS {
        let (Some(a), Some(b)) = (house::room(p.a), house::room(p.b)) else { continue };
        if a.floor == b.floor {
            c.blit(p.ax as i32 + 5, a.floor as i32, DOOR);
            continue;
        }
        let (top, bottom) = (a.floor.min(b.floor) as i32, a.floor.max(b.floor) as i32);
        let x0 = p.ax as i32 - 8;
        for s in 0..8 {
            let (x, y) = (x0 + s * 2, top + s * (bottom - top) / 8);
            c.fill(x, y + 2, 4, 1, 0x4a2e1a);
            c.fill(x, y, 5, 2, 0xc08a4c);
            c.fill(x + 2, y - 8, 1, 8, 0x6a4426);
            c.fill(x, y - 9, 4, 1, 0x8a5a30);
        }
    }

    for (room, x, lift, rows) in DECOR {
        if let Some(r) = house::room(room) {
            c.blit(*x, r.floor as i32 - lift, rows);
        }
    }
    c.0
}

fn brick(x: i32, y: i32) -> u32 {
    let shift = (y / 4) % 2 * 4;
    if y % 4 == 0 || (x + shift) % 8 == 0 { 0x8a7460 } else { 0xa85c44 }
}

pub fn color(c: char) -> Option<u32> {
    Some(match c {
        'k' => 0x1a1420,
        'w' => 0xece4d0,
        'g' => 0x8a8a92,
        'd' => 0x55505a,
        'b' => 0x7a4a26,
        'B' => 0xa8703a,
        'r' => 0xb8402f,
        'o' => 0xe08a2e,
        'y' => 0xe8c84a,
        't' => 0x3fa0a0,
        'u' => 0x3a5aa8,
        'p' => 0x8a4a9a,
        's' => 0xe0b08a,
        'n' => 0x4f8a3f,
        'c' => 0xf4e8c0,
        'R' => 0x7a2a22,
        'G' => 0x2f5a30,
        'T' => 0x7fd0c8,
        'U' => 0x9cc8ee,
        'P' => 0xe890a0,
        'Y' => 0xa88020,
        'D' => 0x33291f,
        'N' => 0x14183a,
        _ => return None,
    })
}

pub const BED: &[&str] = &[
    "RRR                   ",
    "RYR                   ",
    "RYRcccc               ",
    "RYRccccrPrPrPrPrPrPrPR",
    "RYRccccPrPrPrPrPrPrPrR",
    "RRRRRRRRRRRRRRRRRRRRRR",
    "bb                  bb",
];
pub const SPECS: &[&str] = &["kk k kk", "kyk kyk"];
pub const TAP: &[&str] = &[
    "   ggggg  ",
    "   g   g  ",
    "       g  ",
    "wwwwwwwwww",
    " wwwwwwww ",
    "   wwww   ",
    "    gg    ",
    "    gg    ",
    "    gg    ",
    "    gg    ",
    "   gggg   ",
];
pub const DESK: &[&str] = &[
    "BBBBBBBBBBBBBBBBBBBB",
    "bbbbbbbbbbbbbbbbbbbb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
    "bb  bb        bb  bb",
];
pub const STOVE: &[&str] = &[
    "dddddddddddddd",
    "gggggggggggggg",
    "gkkkkkkkkkkkkg",
    "gkddddddddddkg",
    "gkddddddddddkg",
    "gkddddddddddkg",
    "gkkkkkkkkkkkkg",
    "gggggggggggggg",
    "gggggggggggggg",
    "gggggggggggggg",
    "gg          gg",
];
pub const SINK: &[&str] = &[
    "      gg      ",
    "       g      ",
    "wwwwwwwwwwwwww",
    "wtwwwwwwwwwwtw",
    "wtttttttttttww",
    "wwwwwwwwwwwwww",
    "wwwwwwwwwwwwww",
    "wwwwwwwwwwwwww",
    "wwwwwwwwwwwwww",
    "wwwwwwwwwwwwww",
    "ww          ww",
];
pub const KETTLE: &[&str] = &[
    "  ggg  ",
    " g   g ",
    "gggggg ",
    "ggggggg",
    "gggggg ",
];
pub const MUG: &[&str] = &["wwww ", "rrrrw", "wwwww", "wwww "];
pub const PIANO: &[&str] = &[
    "bbbbbbbbbbbbbbbbbbbbbb",
    "BBBBBBBBBBBBBBBBBBBBBB",
    "bbbbbbbbbbbbbbbbbbbbbb",
    "bbbbbbbbbbbbbbbbbbbbbb",
    "wkwkwwkwkwkwwkwkwwkwkw",
    "wwwwwwwwwwwwwwwwwwwwww",
    "bbbbbbbbbbbbbbbbbbbbbb",
    "bb                  bb",
    "bb                  bb",
    "bb                  bb",
    "bb                  bb",
    "bb       gg         bb",
];
pub const CHEST: &[&str] = &[
    "BBBBBBBBBBBBBBBB",
    "bbbbbbbbbbbbbbbb",
    "bbbbbbbyybbbbbbb",
    "BBBBBBByyBBBBBBB",
    "bbbbbbbbbbbbbbbb",
    "bbbbbbbbbbbbbbbb",
    "BBBBBBBBBBBBBBBB",
    "bbbbbbbbbbbbbbbb",
    "bbbbbbbbbbbbbbbb",
    "BBBBBBBBBBBBBBBB",
    "bbbbbbbbbbbbbbbb",
];
pub const TEABAG: &[&str] = &[
    "    y    ",
    "    k    ",
    "  wwwww  ",
    " wwwwwww ",
    " wwgwgww ",
    " wwwwwww ",
    "  wwwww  ",
];

pub const PLAYER: [&[&str]; 2] = [HERO_A, HERO_B];
pub const OTTO: [&[&str]; 2] = [OTTO_A, OTTO_B];

pub const HERO_A: &[&str] = &[
    "  ooooo ", " ooooooo", " ossssoo", "  sksks ", "  sssss ", "   ss   ",
    " nnnnnn ", "snnnnnns", "s nnnn s", "s nnnn s", "  nnnn  ", "  uuuu  ", "  uuuu  ",
    "  uu uu ", "  uu uu ", "  uu uu ", " uu  uu ", " kk  kk ",
];
pub const HERO_B: &[&str] = &[
    "  ooooo ", " ooooooo", " ossssoo", "  sksks ", "  sssss ", "   ss   ",
    " nnnnnn ", "snnnnnns", "s nnnn s", "s nnnn s", "  nnnn  ", "  uuuu  ", "  uuuu  ",
    "  uu uu ", "  uuuu  ", "   uu   ", "   uu   ", "   kk   ",
];
pub const OTTO_A: &[&str] = &[
    "  wwwww ", " wsssssw", " wsksksw", "  sssss ", "  wwwww ", "   www  ",
    " pppppp ", "sppppyps", "s pppp s", "s pppp s", "  pppp  ", "  dddd  ", "  dddd  ",
    "  dd dd ", "  dd dd ", "  dd dd ", " dd  dd ", " kk  kk ",
];
pub const OTTO_B: &[&str] = &[
    "  wwwww ", " wsssssw", " wsksksw", "  sssss ", "  wwwww ", "   www  ",
    " pppppp ", "sppppyps", "s pppp s", "s pppp s", "  pppp  ", "  dddd  ", "  dddd  ",
    "  dd dd ", "  dddd  ", "   dd   ", "   dd   ", "   kk   ",
];

pub const DARKNESS: &[&str] = &[
    "k k k k k k k ",
    " k k k k k k k",
    "kkkkkkkkkkkkk ",
    " kkkkkkkkkkkkk",
    "kkkkkkkkkkkkkk",
    "kkkkkkkkkkkkkk",
    "kkkkkkkkkkkkk ",
    "kkkkkkkkkkkkkk",
    "kkkkkkkkkkkkkk",
    "kkkkkkkkkkkkk ",
    "kkkkkkkkkkkkkk",
    "kkkkkkkkkkkk k",
    "kkkkkkkkkkkkk ",
    "kkkkkkkkkkkk k",
    "kkkkkkkkkkkkk ",
    "kkkkkkkkkkkk k",
    "kkkkkkkkkkk k ",
    "kkkkkkkkkkkk k",
    "kkkkkkkkkkk k ",
    "kkkkkkkkkk k k",
    "kkkkkkkkkkk k ",
    "kkkkkkkkkk k k",
];
/// The heroine of Cheesed! (https://github.com/sunsided/cheesed), small enough for a wall.
pub const POSTER: &[&str] = &[
    "YYYYYYYYYYYYYYY",
    "YNNNNNNNNNNNNNY",
    "YNNNNPPNPPNNNNY",
    "YNNNNPPPPPNNNNY",
    "YNNNyrrrrrrNNNY",
    "YNNyygwkggNNNNY",
    "YNNyoggggggwNNY",
    "YNyyoRggggNNNNY",
    "YNNyRRrggNNNNNY",
    "YNNoRRrggyNNNNY",
    "YNNRRrrgggNNNNY",
    "YNNRRrbbbNNNNNY",
    "YNRRrNbNbNNNNNY",
    "YNRNRNbNbNNNNNY",
    "YNNNNbbNbbNNNNY",
    "YNNNNNNNNNNNNNY",
    "YyyyyyyyyyyyyyY",
    "YYYYYYYYYYYYYYY",
];
pub const DOOR: &[&str] = &[
    "DDDDDDDD",
    "DBBBBBBD", "DBbbbbBD", "DBbBBbBD", "DBbBBbBD", "DBbBBbBD", "DBbBBbBD", "DBbbbbBD",
    "DBBBBBBD", "DBBBBBBD", "DBBBBBBD", "DBBBBByD", "DBBBBBBD", "DBBBBBBD",
    "DBbbbbBD", "DBbBBbBD", "DBbBBbBD", "DBbBBbBD", "DBbBBbBD", "DBbBBbBD", "DBbBBbBD", "DBbbbbBD",
    "DBBBBBBD", "DDDDDDDD",
];
pub const CLOUD: &[&str] = &[
    "      wwwww        ",
    "    wwwwwwwwww     ",
    "  wwwwwwwwwwwwwww  ",
    "wwwwwwwwwwwwwwwwwww",
    "  UUUUUUUUUUUUUUU  ",
];
pub const WINDOW: &[&str] = &[
    "DDDDDDDDDDDDDD",
    "DrrUUUDDUUUrrD",
    "DrrrUUDDUUrrrD",
    "DrrUUUDDUUUrrD",
    "DrUUUUDDUUUUrD",
    "DDDDDDDDDDDDDD",
    "DrUUUUDDUUUUrD",
    "DrrUUUDDUUUrrD",
    "DrUUUUDDUUUUrD",
    "DDDDDDDDDDDDDD",
    "BBBBBBBBBBBBBB",
];
pub const ROUND_WINDOW: &[&str] = &[
    "  DDDDD  ",
    " DUUDUUD ",
    "DUUUDUUUD",
    "DDDDDDDDD",
    "DUUUDUUUD",
    " DUUDUUD ",
    "  DDDDD  ",
];
pub const PICTURE: &[&str] = &[
    "YYYYYYYYYY",
    "YUUUUyUUUY",
    "YUUUUUUUUY",
    "YUUnnUUUUY",
    "YnnnnnnUUY",
    "YnGnnnnnnY",
    "YYYYYYYYYY",
];
pub const CLOCK: &[&str] = &[
    "  YYYYY  ",
    " YwwkwwY ",
    "YwwwkwwwY",
    "YwwwkkkwY",
    "YwwwwwwwY",
    " YwwwwwY ",
    "  YYYYY  ",
];
pub const MIRROR: &[&str] = &[
    " gggggggg ",
    "gTTTwTTTTg",
    "gTTwTTTTTg",
    "gTwTTTTwTg",
    "gTTTTTwTTg",
    "gTTTTwTTTg",
    "gTTTTTTTTg",
    " gggggggg ",
];
pub const TOWEL: &[&str] = &[
    "gggggggggg",
    " PPPPPPPP ",
    " PwwPPwwP ",
    " PPPPPPPP ",
    " PPPPPPPP ",
    " PwwPPwwP ",
    " PPPPPPPP ",
];
pub const NIGHTSTAND: &[&str] = &[
    "BBBBBBBBBB",
    "bbbbbbbbbb",
    "bBBBBBBBBb",
    "bBBByyBBBb",
    "bBBBBBBBBb",
    "bbbbbbbbbb",
    "bBBBBBBBBb",
    "bBBByyBBBb",
    "bBBBBBBBBb",
    "bbbbbbbbbb",
    "b        b",
];
pub const WARDROBE: &[&str] = &[
    "DDDDDDDDDDDDDDDDDD",
    "BBBBBBBBBBBBBBBBBB",
    "bBBBBBBBDDBBBBBBBb",
    "bBbbbbbBDDBbbbbbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbbbbbBDDBbbbbbBb",
    "bBBBBBBBDDBBBBBBBb",
    "bBBBBByBDDByBBBBBb",
    "bBBBBBBBDDBBBBBBBb",
    "bBbbbbbBDDBbbbbbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbBBBbBDDBbBBBbBb",
    "bBbbbbbBDDBbbbbbBb",
    "bBBBBBBBDDBBBBBBBb",
    "DDDDDDDDDDDDDDDDDD",
    "bb              bb",
];
pub const TUB: &[&str] = &[
    "g                           ",
    "gg                          ",
    "gwwwwwwwwwwwwwwwwwwwwwwwwwww",
    "wTTTTTTTTTTTTTTTTTTTTTTTTTTw",
    "wwwwwwwwwwwwwwwwwwwwwwwwwwww",
    "wwwwwwwwwwwwwwwwwwwwwwwwwwww",
    "wwwwwwwwwwwwwwwwwwwwwwwwwwww",
    " wwwwwwwwwwwwwwwwwwwwwwwwww ",
    "  wwwwwwwwwwwwwwwwwwwwwwww  ",
    "  gg                    gg  ",
];
pub const BOOKSHELF: &[&str] = &[
    "DDDDDDDDDDDDDDDD",
    "DrurynprgutPryrD",
    "DrurynprgutPryrD",
    "DrurynprgutPryrD",
    "DrurynprgutPryrD",
    "DrurynprgutPryrD",
    "DbbbbbbbbbbbbbbD",
    "DtpnrPyurnrgutpD",
    "DtpnrPyurnrgutpD",
    "DtpnrPyurnrgutpD",
    "DtpnrPyurnrgutpD",
    "DtpnrPyurnrgutpD",
    "DbbbbbbbbbbbbbbD",
    "DnygrutprPnruygD",
    "DnygrutprPnruygD",
    "DnygrutprPnruygD",
    "DnygrutprPnruygD",
    "DnygrutprPnruygD",
    "DbbbbbbbbbbbbbbD",
    "DurPtgnyrDDDrutD",
    "DurPtgnyrDDDrutD",
    "DurPtgnyrDDDrutD",
    "DurPtgnyrDDDrutD",
    "DurPtgnyrDDDrutD",
    "DbbbbbbbbbbbbbbD",
    "DBBBBBBBBBBBBBBD",
    "DDDDDDDDDDDDDDDD",
];
pub const ARMCHAIR: &[&str] = &[
    "   RRRRRRRRRR   ",
    "  RrrrrrrrrrrR  ",
    "  RrrrrrrrrrrR  ",
    "  RrrrrrrrrrrR  ",
    "  RrrrrrrrrrrR  ",
    "RRRrrrrrrrrrrRRR",
    "RrRRRRRRRRRRRRrR",
    "RrRrrrrrrrrrrRrR",
    "RrRrrrrrrrrrrRrR",
    "RRRRRRRRRRRRRRRR",
    "bb            bb",
];
pub const FLOOR_LAMP: &[&str] = &[
    "  yyy  ",
    " yyyyy ",
    "yyyyyyy",
    "YYYYYYY",
    "   d   ", "   d   ", "   d   ", "   d   ", "   d   ", "   d   ",
    "   d   ", "   d   ", "   d   ", "   d   ", "   d   ", "   d   ",
    "   d   ", "   d   ", "   d   ", "   d   ", "   d   ", "   d   ",
    "  ddd  ",
    " ddddd ",
];
pub const CUPBOARD: &[&str] = &[
    "DDDDDDDDDDDDDDDDDDDDDD",
    "GnnnnnnnnnGGnnnnnnnnnG",
    "GnGGGGGGGnGGnGGGGGGGnG",
    "GnGnnnnnGnGGnGnnnnnGnG",
    "GnGnnnnnGnGGnGnnnnnGnG",
    "GnGGGGGGGnGGnGGGGGGGnG",
    "GnnnnnnnynGGynnnnnnnnG",
    "DDDDDDDDDDDDDDDDDDDDDD",
];
pub const JARS: &[&str] = &[
    " ww   rr   ww   yy ",
    "yyyy rrrr nnnn oooo",
    "yyyy rrrr nnnn oooo",
    "yyyy rrrr nnnn oooo",
    "BBBBBBBBBBBBBBBBBBB",
    "b                 b",
];
pub const TRUNK: &[&str] = &[
    "  bbbbbbbbbbbbbbb  ",
    " bBBBBBBBBBBBBBBBb ",
    "bBBBBBBBBBBBBBBBBBb",
    "yyyyyyyyyyyyyyyyyyy",
    "bBBBBBByyyBBBBBBBBb",
    "bBBBBBByyyBBBBBBBBb",
    "bBBBBBBBBBBBBBBBBBb",
    "yyyyyyyyyyyyyyyyyyy",
    "bBBBBBBBBBBBBBBBBBb",
];
pub const BOXES: &[&str] = &[
    "     BBBBBBBB  ",
    "     byyyyyyb  ",
    "     bBBBBBBb  ",
    "     bBBBBBBb  ",
    "BBBBBBBBBBBBBBB",
    "byyyyyybBBBBBBb",
    "bBBBBBBbyyyyyyb",
    "bBBBBBBbBBBBBBb",
    "bBBBBBBbBBBBBBb",
    "bbbbbbbbbbbbbbb",
];
pub const BARREL: &[&str] = &[
    " bbbbbbbb ",
    "gggggggggg",
    "bBbBbBbBbB",
    "bBbBbBbBbB",
    "gggggggggg",
    "bBbBbBbBbB",
    "bBbBbBbBbB",
    "bBbBbBbBbB",
    "gggggggggg",
    "bBbBbBbBbB",
    "bBbBbBbBbB",
    "gggggggggg",
    " bbbbbbbb ",
];
pub const COAL: &[&str] = &[
    "      dk       ",
    "    kdkkdk     ",
    "  kkdkkkkdkk   ",
    " kdkkkdkkkkdk  ",
    "kkkdkkkkdkkkkdk",
];
pub const BOILER: &[&str] = &[
    "    gg      ",
    "    gg      ",
    "   gggg     ",
    "  dddddddd  ",
    " dggggggggd ",
    " dgddddddgd ",
    " dgdyyyydgd ",
    " dgdyoyydgd ",
    " dgddddddgd ",
    " dggggggggd ",
    " dggggggggd ",
    " dggrrggggd ",
    " dggggggggd ",
    " dggggggggd ",
    " dddddddddd ",
    " dkkkkkkkkd ",
    " dkoorookkd ",
    " dkkkkkkkkd ",
    " dddddddddd ",
    "  dd    dd  ",
];

/// Smoke from the chimney, the only part of the background that moves.
pub fn smoke(t: f32) -> [([f32; 2], f32); 4] {
    std::array::from_fn(|i| {
        let phase = (t * 0.25 + i as f32 * 0.25).fract();
        ([(276.0 + (phase * 9.0).sin() * 3.0).round(), (15.0 - phase * 15.0).round()], 0.6 * (1.0 - phase))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A letter missing from the palette would silently draw as a hole.
    #[test]
    fn every_sprite_uses_the_palette() {
        let sprites = DECOR.iter().map(|d| d.3)
            .chain(house::SPOTS.iter().map(|s| s.sprite))
            .chain(PLAYER).chain(OTTO).chain([DOOR, CLOUD]);
        for rows in sprites {
            for ch in rows.concat().chars().filter(|c| *c != ' ') {
                assert!(color(ch).is_some(), "{ch} in {rows:?}");
            }
        }
    }
}
