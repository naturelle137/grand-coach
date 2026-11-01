// SPDX-License-Identifier: EUPL-1.2

//! Generates the card faces of Grand-Coach as SVG (D-14): a four-colour
//! French tournament deck, a classic two-colour French deck and a German
//! deck in the manner of the Altenburger Bild. Every shape is drawn here;
//! indices are stroked paths, so no font is needed to render them.
//!
//! Every deck also has a Mephisto variant (D-41): the Jacks (Unter) carry
//! the index M and a devil, and the Kreuz (Eichel) cards an inverted cross
//! as suit symbol. The Mephisto back shows flames in the gate to hell.
//!
//! Every deck also has a God Mode variant (D-62): the Jacks carry the
//! index G and an angel, and the suits carry light symbols: an upright
//! Latin cross (Kreuz, Eichel), a crescent moon (Pik, Grün), a radiant
//! heart (Herz, Rot) and a star of light (Karo, Schellen). The God Mode
//! back is bright, with the all-seeing eye in rays of light.
//!
//! Output paths are relative to `assets/cards/`: `<theme><deck>/<suit>-<rank>.svg`
//! and `<theme>back.svg`, where `<theme>` is empty, `mephisto/` or `god/`.
//! Generated files are CC BY-SA 4.0 (`LICENSE-ASSETS`).

use std::fmt::Write;

/// Card decks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deck {
    /// French faces, four colours (default).
    French4,
    /// French faces, two colours.
    French2,
    /// German faces (Eichel, Grün, Rot, Schellen).
    German,
}

impl Deck {
    /// All decks.
    pub const ALL: [Deck; 3] = [Deck::French4, Deck::French2, Deck::German];

    /// Directory name below `assets/cards/`.
    pub const fn dir(self) -> &'static str {
        match self {
            Deck::French4 => "french4",
            Deck::French2 => "french2",
            Deck::German => "german",
        }
    }
}

/// Variants of every deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    /// The ordinary faces and back.
    Classic,
    /// Against Mephisto (D-41).
    Mephisto,
    /// In God Mode (D-62).
    God,
}

impl Theme {
    /// All themes.
    pub const ALL: [Theme; 3] = [Theme::Classic, Theme::Mephisto, Theme::God];

    /// Directory prefix below `assets/cards/`.
    pub const fn prefix(self) -> &'static str {
        match self {
            Theme::Classic => "",
            Theme::Mephisto => "mephisto/",
            Theme::God => "god/",
        }
    }
}

/// Suit names used in file names, in `skat-core` order.
pub const SUITS: [&str; 4] = ["kreuz", "pik", "herz", "karo"];
/// Rank names used in file names, in `skat-core` order.
pub const RANKS: [&str; 8] = [
    "sieben", "acht", "neun", "zehn", "bube", "dame", "koenig", "ass",
];

const W: f32 = 200.0;
const H: f32 = 300.0;
const LICENSE: &str = "<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->";

/// All generated files as (path below `assets/cards/`, SVG text).
pub fn generate() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for theme in Theme::ALL {
        let prefix = theme.prefix();
        for deck in Deck::ALL {
            for (s, suit) in SUITS.iter().enumerate() {
                for (r, rank) in RANKS.iter().enumerate() {
                    let path = format!("{prefix}{}/{suit}-{rank}.svg", deck.dir());
                    files.push((path, card(deck, s, r, theme)));
                }
            }
        }
    }
    files.push(("back.svg".to_owned(), back()));
    files.push(("mephisto/back.svg".to_owned(), mephisto_back()));
    files.push(("god/back.svg".to_owned(), god_back()));
    files
}

fn svg(body: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {W} {H}\" width=\"{W}\" height=\"{H}\">\n\
         {LICENSE}\n{body}</svg>\n"
    )
}

fn outline(fill: &str) -> String {
    format!(
        "<rect x=\"2\" y=\"2\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"{fill}\" stroke=\"#333\" stroke-width=\"2\"/>\n",
        W - 4.0,
        H - 4.0
    )
}

/// The back, shared by all decks.
fn back() -> String {
    let mut body = outline("#ffffff");
    body.push_str(
        "<defs><pattern id=\"lattice\" width=\"16\" height=\"16\" patternUnits=\"userSpaceOnUse\">\
         <rect width=\"16\" height=\"16\" fill=\"#24489a\"/>\
         <path d=\"M0,8 L8,0 L16,8 L8,16 Z\" fill=\"none\" stroke=\"#9fb4e8\" stroke-width=\"1.5\"/>\
         </pattern></defs>\n",
    );
    body.push_str(
        "<rect x=\"12\" y=\"12\" width=\"176\" height=\"276\" rx=\"6\" fill=\"url(#lattice)\" stroke=\"#24489a\" stroke-width=\"2\"/>\n",
    );
    body.push_str(
        "<rect x=\"60\" y=\"110\" width=\"80\" height=\"80\" rx=\"10\" fill=\"#24489a\" stroke=\"#9fb4e8\" stroke-width=\"3\"/>\n",
    );
    body.push_str(&glyphs("GC", 74.0, 132.0, 0.6, "#ffffff"));
    svg(&body)
}

/// The Mephisto back, shared by all decks: flames in the gate to hell.
fn mephisto_back() -> String {
    let mut body = outline("#1a0404");
    body.push_str(
        "<defs><pattern id=\"embers\" width=\"16\" height=\"16\" patternUnits=\"userSpaceOnUse\">\
         <rect width=\"16\" height=\"16\" fill=\"#3a0606\"/>\
         <path d=\"M0,8 L8,0 L16,8 L8,16 Z\" fill=\"none\" stroke=\"#8a1a10\" stroke-width=\"1.5\"/>\
         </pattern>\
         <radialGradient id=\"glow\" cx=\"0.5\" cy=\"0.85\" r=\"0.8\">\
         <stop offset=\"0\" stop-color=\"#ff8c1a\"/><stop offset=\"0.45\" stop-color=\"#9a1408\"/>\
         <stop offset=\"1\" stop-color=\"#120202\"/></radialGradient>\
         <clipPath id=\"opening\"><path d=\"M58,266 L58,135 Q58,80 100,62 Q142,80 142,135 L142,266 Z\"/></clipPath>\
         </defs>\n",
    );
    body.push_str(
        "<rect x=\"12\" y=\"12\" width=\"176\" height=\"276\" rx=\"6\" fill=\"url(#embers)\" stroke=\"#8a1a10\" stroke-width=\"2\"/>\n",
    );
    // The gate: a pointed stone arch with joints, a keystone with horns
    // and a step below.
    body.push_str(
        "<path d=\"M38,266 L38,128 Q38,58 100,38 Q162,58 162,128 L162,266 Z\" fill=\"#4a3434\" stroke=\"#120202\" stroke-width=\"3\"/>\n\
         <path d=\"M38,170 L58,170 M38,218 L58,218 M142,170 L162,170 M142,218 L162,218 \
         M40,124 L58,128 M160,124 L142,128 M56,74 L70,90 M144,74 L130,90\" stroke=\"#120202\" stroke-width=\"2\"/>\n\
         <path d=\"M90,40 L110,40 L106,66 L94,66 Z\" fill=\"#5e4444\" stroke=\"#120202\" stroke-width=\"2\"/>\n\
         <path d=\"M92,42 Q82,30 86,20 Q90,32 97,38 Z M108,42 Q118,30 114,20 Q110,32 103,38 Z\" fill=\"#7a0a0a\" stroke=\"#120202\" stroke-width=\"1.5\"/>\n\
         <rect x=\"28\" y=\"264\" width=\"144\" height=\"14\" rx=\"2\" fill=\"#3a2828\" stroke=\"#120202\" stroke-width=\"2\"/>\n",
    );
    // The opening with the fire inside.
    body.push_str(
        "<g clip-path=\"url(#opening)\">\
         <rect x=\"58\" y=\"60\" width=\"84\" height=\"206\" fill=\"url(#glow)\"/>\
         <path d=\"M56,268 L56,220 Q66,190 70,200 Q72,160 84,140 Q84,170 92,175 Q94,130 104,96 Q110,140 116,150 \
         Q122,130 128,118 Q128,165 134,180 Q140,170 144,190 L144,268 Z\" fill=\"#d42010\"/>\
         <path d=\"M62,268 Q66,235 74,225 Q76,200 86,185 Q88,205 94,208 Q98,170 106,142 Q110,180 118,190 \
         Q122,175 126,165 Q128,200 136,215 Q142,240 138,268 Z\" fill=\"#f07a10\"/>\
         <path d=\"M74,268 Q76,245 86,235 Q90,220 96,205 Q100,225 104,228 Q108,205 114,195 Q118,225 124,240 \
         Q130,255 126,268 Z\" fill=\"#ffd23a\"/>\
         <circle cx=\"80\" cy=\"120\" r=\"2\" fill=\"#ffb020\"/><circle cx=\"122\" cy=\"100\" r=\"1.5\" fill=\"#ffb020\"/>\
         <circle cx=\"96\" cy=\"84\" r=\"1.5\" fill=\"#ffd23a\"/>\
         </g>\n\
         <path d=\"M58,266 L58,135 Q58,80 100,62 Q142,80 142,135 L142,266\" fill=\"none\" stroke=\"#120202\" stroke-width=\"3\"/>\n",
    );
    svg(&body)
}

/// The God Mode back, shared by all decks: the all-seeing eye in a
/// triangle, surrounded by rays of light on a bright sky.
fn god_back() -> String {
    let mut body = outline("#fffdf4");
    body.push_str(
        "<defs><pattern id=\"sky\" width=\"16\" height=\"16\" patternUnits=\"userSpaceOnUse\">\
         <rect width=\"16\" height=\"16\" fill=\"#d8e9fb\"/>\
         <path d=\"M0,8 L8,0 L16,8 L8,16 Z\" fill=\"none\" stroke=\"#ffffff\" stroke-width=\"1.5\"/>\
         </pattern>\
         <radialGradient id=\"light\" cx=\"0.5\" cy=\"0.5\" r=\"0.5\">\
         <stop offset=\"0\" stop-color=\"#fffbe0\"/><stop offset=\"0.6\" stop-color=\"#fff1a8\" stop-opacity=\"0.7\"/>\
         <stop offset=\"1\" stop-color=\"#fff1a8\" stop-opacity=\"0\"/></radialGradient>\
         <clipPath id=\"inner\"><rect x=\"12\" y=\"12\" width=\"176\" height=\"276\" rx=\"6\"/></clipPath>\
         </defs>\n",
    );
    body.push_str(
        "<rect x=\"12\" y=\"12\" width=\"176\" height=\"276\" rx=\"6\" fill=\"url(#sky)\" stroke=\"#d8a820\" stroke-width=\"2\"/>\n",
    );
    // Rays of light from the centre.
    body.push_str("<g clip-path=\"url(#inner)\">");
    for k in 0..24u8 {
        let a = (f32::from(k) * 15.0).to_radians();
        let b = a + 5.0_f32.to_radians();
        let _ = write!(
            body,
            "<path d=\"M100,150 L{:.1},{:.1} L{:.1},{:.1} Z\" fill=\"#ffe88a\" fill-opacity=\"0.55\"/>",
            100.0 + a.cos() * 200.0,
            150.0 + a.sin() * 200.0,
            100.0 + b.cos() * 200.0,
            150.0 + b.sin() * 200.0
        );
    }
    body.push_str("<circle cx=\"100\" cy=\"150\" r=\"80\" fill=\"url(#light)\"/></g>\n");
    // The eye in a triangle.
    body.push_str(
        "<path d=\"M100,92 L152,184 L48,184 Z\" fill=\"#fff4c0\" stroke=\"#c8961a\" stroke-width=\"4\" stroke-linejoin=\"round\"/>\n\
         <path d=\"M70,154 Q100,128 130,154 Q100,180 70,154 Z\" fill=\"#ffffff\" stroke=\"#5a4410\" stroke-width=\"3\"/>\n\
         <circle cx=\"100\" cy=\"154\" r=\"11\" fill=\"#3a7ad8\" stroke=\"#1e4a8a\" stroke-width=\"2\"/>\n\
         <circle cx=\"100\" cy=\"154\" r=\"5\" fill=\"#10203a\"/><circle cx=\"97\" cy=\"150\" r=\"2\" fill=\"#ffffff\"/>\n\
         <path d=\"M72,142 Q100,118 128,142\" fill=\"none\" stroke=\"#5a4410\" stroke-width=\"2.5\" stroke-linecap=\"round\"/>\n",
    );
    svg(&body)
}

/// Colours of a suit: fill, outline and index ink.
struct Ink {
    fill: &'static str,
    stroke: &'static str,
    index: &'static str,
}

fn ink(deck: Deck, suit: usize) -> Ink {
    const BLACK: Ink = Ink {
        fill: "#1a1a1a",
        stroke: "#1a1a1a",
        index: "#1a1a1a",
    };
    const RED: Ink = Ink {
        fill: "#d42020",
        stroke: "#a01010",
        index: "#d42020",
    };
    const GREEN: Ink = Ink {
        fill: "#1e8a32",
        stroke: "#145c22",
        index: "#1e8a32",
    };
    const YELLOW: Ink = Ink {
        fill: "#f0a800",
        stroke: "#8a5a00",
        index: "#b07800",
    };
    const BROWN: Ink = Ink {
        fill: "#7a4e1c",
        stroke: "#3e2608",
        index: "#6a400e",
    };
    match (deck, suit) {
        (Deck::French4, 0) | (Deck::French2, 0 | 1) => BLACK,
        (Deck::French4, 1) | (Deck::German, 1) => GREEN,
        (Deck::French4, 3) | (Deck::German, 3) => YELLOW,
        (Deck::German, 0) => BROWN,
        _ => RED,
    }
}

/// Suit symbol drawn in a 100 × 100 box centred on the origin. `flipped`
/// marks a symbol that is drawn rotated by 180°; the Mephisto cross is
/// then drawn upright, so it appears inverted on the card, and the God
/// Mode cross inverted, so it always appears upright.
fn symbol(deck: Deck, suit: usize, theme: Theme, flipped: bool) -> String {
    let i = ink(deck, suit);
    let (fill, stroke) = (i.fill, i.stroke);
    if theme == Theme::God {
        return light_symbol(suit, fill, stroke, flipped);
    }
    if theme == Theme::Mephisto && suit == 0 {
        // Inverted Latin cross: the crossbar sits below the centre.
        let d = if flipped {
            "M-7,-48 H7 V-26 H30 V-12 H7 V48 H-7 V-12 H-30 V-26 H-7 Z"
        } else {
            "M-7,-48 H7 V12 H30 V26 H7 V48 H-7 V26 H-30 V12 H-7 Z"
        };
        return format!(
            "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"3\" stroke-linejoin=\"round\"/>"
        );
    }
    let path = |d: &str| {
        format!(
            "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"3\" stroke-linejoin=\"round\"/>"
        )
    };
    match (deck, suit) {
        (Deck::German, 0) => format!(
            "<rect x=\"-4\" y=\"-50\" width=\"8\" height=\"14\" fill=\"{stroke}\"/>\
             <path d=\"M-20,-14 C-24,16 -10,44 0,47 C10,44 24,16 20,-14 Z\" fill=\"#c8923e\" stroke=\"{stroke}\" stroke-width=\"3\"/>\
             <path d=\"M-28,-12 C-28,-42 28,-42 28,-12 Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"3\"/>\
             <path d=\"M-18,-30 L-8,-14 M-4,-36 L6,-14 M10,-34 L18,-16\" stroke=\"#c8923e\" stroke-width=\"2.5\" fill=\"none\"/>"
        ),
        (Deck::German, 1) => format!(
            "{}<path d=\"M0,-36 V48 M0,-10 L-16,-22 M0,-10 L16,-22 M0,10 L-20,-2 M0,10 L20,-2\" stroke=\"{stroke}\" stroke-width=\"3\" fill=\"none\"/>",
            path(
                "M0,-48 C26,-34 42,-6 32,18 C24,34 10,40 0,44 C-10,40 -24,34 -32,18 C-42,-6 -26,-34 0,-48 Z"
            )
        ),
        (Deck::German, 3) => format!(
            "<circle cx=\"0\" cy=\"-40\" r=\"7\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"4\"/>\
             <circle cx=\"0\" cy=\"4\" r=\"38\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"3\"/>\
             <path d=\"M-37,-6 H37\" stroke=\"#c01818\" stroke-width=\"6\"/>\
             <path d=\"M-16,24 H16\" stroke=\"{stroke}\" stroke-width=\"4\"/>\
             <circle cx=\"0\" cy=\"31\" r=\"5\" fill=\"{stroke}\"/>"
        ),
        (_, 0) => format!(
            "<circle cx=\"0\" cy=\"-24\" r=\"19\" fill=\"{fill}\"/><circle cx=\"-22\" cy=\"6\" r=\"19\" fill=\"{fill}\"/>\
             <circle cx=\"22\" cy=\"6\" r=\"19\" fill=\"{fill}\"/><circle cx=\"0\" cy=\"0\" r=\"10\" fill=\"{fill}\"/>\
             <path d=\"M-5,4 C-4,24 -10,38 -20,46 H20 C10,38 4,24 5,4 Z\" fill=\"{fill}\"/>"
        ),
        (_, 1) => path(
            "M0,-46 C10,-30 46,-12 46,10 C46,30 22,36 6,22 C8,34 12,42 20,46 H-20 C-12,42 -8,34 -6,22 C-22,36 -46,30 -46,10 C-46,-12 -10,-30 0,-46 Z",
        ),
        (_, 2) => path(HEART),
        _ => path(
            "M0,-48 C10,-30 22,-14 34,0 C22,14 10,30 0,48 C-10,30 -22,14 -34,0 C-22,-14 -10,-30 0,-48 Z",
        ),
    }
}

/// The God Mode suit symbols (D-62) in a 100 × 100 box centred on the
/// origin, for every deck: Latin cross, crescent moon, radiant heart and
/// star of light.
fn light_symbol(suit: usize, fill: &str, stroke: &str, flipped: bool) -> String {
    let path = |d: &str| {
        format!(
            "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"3\" stroke-linejoin=\"round\"/>"
        )
    };
    match suit {
        // Upright Latin cross: the crossbar sits above the centre. A
        // flipped symbol is drawn inverted so it appears upright.
        0 if flipped => path("M-7,-48 H7 V12 H30 V26 H7 V48 H-7 V26 H-30 V12 H-7 Z"),
        0 => path("M-7,-48 H7 V-26 H30 V-12 H7 V48 H-7 V-12 H-30 V-26 H-7 Z"),
        // Crescent moon, open to the upper right.
        1 => path("M5,-43.7 A44,44 0 1,0 35.8,25.6 A38,38 0 1,1 5,-43.7 Z"),
        // Heart in a ring of rays.
        2 => {
            let mut out = String::new();
            for k in 0..8u8 {
                let a = (f32::from(k) * 45.0 + 22.5).to_radians();
                let (x, y) = (a.cos(), a.sin());
                let _ = write!(
                    out,
                    "<path d=\"M{:.1},{:.1} L{:.1},{:.1}\" stroke=\"{stroke}\" stroke-width=\"4\" stroke-linecap=\"round\"/>",
                    x * 38.0,
                    y * 38.0,
                    x * 48.0,
                    y * 48.0
                );
            }
            out.push_str(&format!("<g transform=\"scale(0.72)\">{}</g>", path(HEART)));
            out
        }
        // Star of light: four long and four short rays.
        _ => {
            let mut d = String::new();
            for k in 0..16u8 {
                let r = match k % 4 {
                    0 => 48.0,
                    2 => 24.0,
                    _ => 9.0,
                };
                let a = (f32::from(k) * 22.5 - 90.0).to_radians();
                let _ = write!(
                    d,
                    "{}{:.1},{:.1} ",
                    if k == 0 { "M" } else { "L" },
                    a.cos() * r,
                    a.sin() * r
                );
            }
            d.push('Z');
            path(&d)
        }
    }
}

const HEART: &str =
    "M0,42 C-8,32 -46,8 -46,-16 C-46,-38 -18,-46 0,-24 C18,-46 46,-38 46,-16 C46,8 8,32 0,42 Z";

fn place(content: &str, x: f32, y: f32, scale: f32, rotate: bool) -> String {
    let r = if rotate { " rotate(180)" } else { "" };
    format!("<g transform=\"translate({x},{y}) scale({scale}){r}\">{content}</g>\n")
}

/// Stroked glyph in a 40 × 60 cell, as (advance, SVG).
fn glyph(c: char, color: &str) -> (f32, String) {
    let stroke = format!(
        "fill=\"none\" stroke=\"{color}\" stroke-width=\"7\" stroke-linecap=\"round\" stroke-linejoin=\"round\""
    );
    let path = |d: &str| format!("<path d=\"{d}\" {stroke}/>");
    let ellipse = |cx: u8, cy: u8, rx: u8, ry: u8| {
        format!("<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" {stroke}/>")
    };
    match c {
        '7' => (44.0, path("M3,4 H37 L15,57")),
        '8' => (
            44.0,
            format!("{}{}", ellipse(20, 16, 12, 12), ellipse(20, 43, 15, 14)),
        ),
        '9' => (
            44.0,
            format!(
                "{}{}",
                ellipse(20, 20, 15, 16),
                path("M35,20 C35,42 26,56 8,57")
            ),
        ),
        '1' => (24.0, path("M4,14 L14,4 V57")),
        '0' => (44.0, ellipse(20, 30, 15, 27)),
        'B' => (
            44.0,
            path("M7,3 V57 H24 C44,57 44,29 24,29 H7 M7,3 H21 C38,3 38,29 21,29"),
        ),
        'D' => (44.0, path("M7,3 V57 H17 C45,57 45,3 17,3 Z")),
        'K' => (44.0, path("M7,3 V57 M35,3 L7,37 M17,26 L37,57")),
        'A' => (44.0, path("M3,57 L20,3 L37,57 M9,39 H31")),
        'M' => (44.0, path("M5,57 V3 L20,34 L35,3 V57")),
        'U' => (44.0, path("M6,3 V38 C6,62 34,62 34,38 V3")),
        'O' => (44.0, ellipse(20, 30, 16, 27)),
        'G' => (
            44.0,
            path("M36,10 C30,2 6,0 4,30 C6,60 34,60 36,40 V32 H22"),
        ),
        'C' => (44.0, path("M36,10 C30,2 6,0 4,30 C6,60 34,60 36,50")),
        _ => panic!("no glyph for {c}"),
    }
}

/// A string of glyphs with its top-left corner at (x, y).
fn glyphs(text: &str, x: f32, y: f32, scale: f32, color: &str) -> String {
    let mut out = String::new();
    let mut advance = 0.0;
    for c in text.chars() {
        let (width, svg) = glyph(c, color);
        out.push_str(&place(&svg, x + advance * scale, y, scale, false));
        advance += width;
    }
    out
}

fn index_text(deck: Deck, rank: usize, theme: Theme) -> &'static str {
    match (deck, rank) {
        (_, 4) if theme == Theme::Mephisto => "M",
        (_, 4) if theme == Theme::God => "G",
        (_, 0) => "7",
        (_, 1) => "8",
        (_, 2) => "9",
        (_, 3) => "10",
        (Deck::German, 4) => "U",
        (Deck::German, 5) => "O",
        (_, 4) => "B",
        (_, 5) => "D",
        (_, 6) => "K",
        _ => "A",
    }
}

/// Pip positions of the number cards, from the top.
fn pips(rank: usize) -> &'static [(f32, f32)] {
    match rank {
        0 => &[
            (66.0, 72.0),
            (134.0, 72.0),
            (100.0, 111.0),
            (66.0, 150.0),
            (134.0, 150.0),
            (66.0, 228.0),
            (134.0, 228.0),
        ],
        1 => &[
            (66.0, 72.0),
            (134.0, 72.0),
            (100.0, 111.0),
            (66.0, 150.0),
            (134.0, 150.0),
            (100.0, 189.0),
            (66.0, 228.0),
            (134.0, 228.0),
        ],
        2 => &[
            (66.0, 72.0),
            (134.0, 72.0),
            (66.0, 124.0),
            (134.0, 124.0),
            (100.0, 150.0),
            (66.0, 176.0),
            (134.0, 176.0),
            (66.0, 228.0),
            (134.0, 228.0),
        ],
        _ => &[
            (66.0, 72.0),
            (134.0, 72.0),
            (100.0, 98.0),
            (66.0, 124.0),
            (134.0, 124.0),
            (66.0, 176.0),
            (134.0, 176.0),
            (100.0, 202.0),
            (66.0, 228.0),
            (134.0, 228.0),
        ],
    }
}

/// The upper half of the Mephisto Jack: a devil with horns and a goatee.
fn devil(deck: Deck, suit: usize, flipped: bool) -> String {
    let i = ink(deck, suit);
    let mut out = format!(
        "<path d=\"M66,150 L76,116 Q100,104 124,116 L134,150 Z\" fill=\"{}\" stroke=\"#333\" stroke-width=\"2\"/>\
         <path d=\"M88,108 L100,128 L112,108\" fill=\"#7a0a0a\" stroke=\"#333\" stroke-width=\"2\"/>\
         <path d=\"M86,84 Q76,66 80,54 Q84,70 94,78 Z\" fill=\"#3a0606\" stroke=\"#222\" stroke-width=\"2\"/>\
         <path d=\"M114,84 Q124,66 120,54 Q116,70 106,78 Z\" fill=\"#3a0606\" stroke=\"#222\" stroke-width=\"2\"/>\
         <path d=\"M84,92 Q84,76 100,76 Q116,76 116,92 Q116,104 106,110 L100,122 L94,110 Q84,104 84,92 Z\" fill=\"#c8102e\" stroke=\"#333\" stroke-width=\"2\"/>\
         <path d=\"M89,88 L97,91 M111,88 L103,91\" stroke=\"#1a1a1a\" stroke-width=\"2.5\" stroke-linecap=\"round\"/>\
         <circle cx=\"94\" cy=\"94\" r=\"2\" fill=\"#f0d020\"/><circle cx=\"106\" cy=\"94\" r=\"2\" fill=\"#f0d020\"/>\
         <path d=\"M93,103 Q100,108 107,103\" fill=\"none\" stroke=\"#1a1a1a\" stroke-width=\"2\" stroke-linecap=\"round\"/>",
        i.fill
    );
    out.push_str(&place(
        &symbol(deck, suit, Theme::Mephisto, flipped),
        64.0,
        78.0,
        0.2,
        false,
    ));
    out
}

/// The upper half of the God Mode Jack: an angel with a halo and wings.
fn angel(deck: Deck, suit: usize, flipped: bool) -> String {
    let i = ink(deck, suit);
    let mut out = format!(
        "<path d=\"M74,124 Q50,104 52,74 Q64,96 82,112 Z\" fill=\"#ffffff\" stroke=\"#8a8a9a\" stroke-width=\"2\"/>\
         <path d=\"M126,124 Q150,104 148,74 Q136,96 118,112 Z\" fill=\"#ffffff\" stroke=\"#8a8a9a\" stroke-width=\"2\"/>\
         <path d=\"M60,92 Q66,100 72,104 M140,92 Q134,100 128,104\" fill=\"none\" stroke=\"#8a8a9a\" stroke-width=\"1.5\"/>\
         <path d=\"M66,150 L76,116 Q100,104 124,116 L134,150 Z\" fill=\"{}\" stroke=\"#333\" stroke-width=\"2\"/>\
         <path d=\"M90,112 L100,130 L110,112\" fill=\"#ffffff\" stroke=\"#333\" stroke-width=\"2\"/>\
         <circle cx=\"100\" cy=\"94\" r=\"16\" fill=\"#f6d7b0\" stroke=\"#333\" stroke-width=\"2\"/>\
         <path d=\"M84,92 Q84,74 100,74 Q116,74 116,92 Q110,82 100,82 Q90,82 84,92 Z\" fill=\"#e8c060\" stroke=\"#333\" stroke-width=\"1.5\"/>\
         <ellipse cx=\"100\" cy=\"68\" rx=\"18\" ry=\"5\" fill=\"none\" stroke=\"#e8b820\" stroke-width=\"4\"/>\
         <circle cx=\"94\" cy=\"95\" r=\"1.8\" fill=\"#1a1a1a\"/><circle cx=\"106\" cy=\"95\" r=\"1.8\" fill=\"#1a1a1a\"/>\
         <path d=\"M94,102 Q100,106 106,102\" fill=\"none\" stroke=\"#1a1a1a\" stroke-width=\"2\" stroke-linecap=\"round\"/>",
        i.fill
    );
    out.push_str(&place(
        &symbol(deck, suit, Theme::God, flipped),
        64.0,
        78.0,
        0.2,
        false,
    ));
    out
}

/// One half of a court figure (the upper one; the lower is the same half
/// rotated, drawn with `flipped`).
fn figure(deck: Deck, suit: usize, rank: usize, theme: Theme, flipped: bool) -> String {
    match (theme, rank) {
        (Theme::Mephisto, 4) => return devil(deck, suit, flipped),
        (Theme::God, 4) => return angel(deck, suit, flipped),
        _ => {}
    }
    let i = ink(deck, suit);
    let mut out = format!(
        "<path d=\"M66,150 L76,116 Q100,104 124,116 L134,150 Z\" fill=\"{}\" stroke=\"#333\" stroke-width=\"2\"/>\
         <path d=\"M90,112 L100,130 L110,112\" fill=\"#ffffff\" stroke=\"#333\" stroke-width=\"2\"/>\
         <circle cx=\"100\" cy=\"94\" r=\"16\" fill=\"#f6d7b0\" stroke=\"#333\" stroke-width=\"2\"/>",
        i.fill
    );
    let headwear = match (deck, rank) {
        // King: crown.
        (_, 6) => "<path d=\"M84,82 L86,62 L93,72 L100,58 L107,72 L114,62 L116,82 Z\" fill=\"#e8b820\" stroke=\"#333\" stroke-width=\"2\"/>".to_owned(),
        // Ober: hat with a feather.
        (Deck::German, 5) => format!(
            "<path d=\"M78,84 Q100,62 122,84 Z\" fill=\"{}\" stroke=\"#333\" stroke-width=\"2\"/>\
             <path d=\"M112,74 Q128,52 136,58\" fill=\"none\" stroke=\"#c01818\" stroke-width=\"4\" stroke-linecap=\"round\"/>",
            i.stroke
        ),
        // Dame: veil.
        (_, 5) => "<path d=\"M82,104 Q76,70 100,72 Q124,70 118,104 Q112,86 100,84 Q88,86 82,104 Z\" fill=\"#6a3a8a\" stroke=\"#333\" stroke-width=\"2\"/>".to_owned(),
        // Bube / Unter: cap.
        _ => format!(
            "<path d=\"M82,86 Q98,64 120,80 L126,86 Z\" fill=\"{}\" stroke=\"#333\" stroke-width=\"2\"/>",
            i.stroke
        ),
    };
    out.push_str(&headwear);
    out.push_str(&place(
        &symbol(deck, suit, theme, flipped),
        64.0,
        78.0,
        0.2,
        false,
    ));
    out
}

fn card(deck: Deck, suit: usize, rank: usize, theme: Theme) -> String {
    let i = ink(deck, suit);
    let mut body = outline("#ffffff");
    let sym = symbol(deck, suit, theme, false);
    let flipped = symbol(deck, suit, theme, true);
    // Corner indices, top left and rotated bottom right.
    let index = glyphs(index_text(deck, rank, theme), 12.0, 14.0, 0.42, i.index);
    body.push_str(&index);
    body.push_str(&place(&sym, 24.0, 56.0, 0.2, false));
    body.push_str(&format!(
        "<g transform=\"rotate(180 {} {})\">\n{index}{}</g>\n",
        W / 2.0,
        H / 2.0,
        place(&flipped, 24.0, 56.0, 0.2, false)
    ));
    match rank {
        0..=3 => {
            for &(x, y) in pips(rank) {
                let rotated = y > 150.0;
                let pip = if rotated { &flipped } else { &sym };
                body.push_str(&place(pip, x, y, 0.34, rotated));
            }
        }
        7 => body.push_str(&place(&sym, 100.0, 150.0, 0.9, false)),
        _ => {
            let tint = match i.fill {
                "#1a1a1a" => "#ececec",
                "#d42020" => "#fbe3e3",
                "#1e8a32" => "#e2f3e5",
                "#f0a800" => "#fdf1d2",
                _ => "#f1e6d8",
            };
            let _ = write!(
                body,
                "<rect x=\"44\" y=\"40\" width=\"112\" height=\"220\" rx=\"6\" fill=\"{tint}\" stroke=\"{}\" stroke-width=\"3\"/>\n\
                 <clipPath id=\"frame\"><rect x=\"44\" y=\"40\" width=\"112\" height=\"220\" rx=\"6\"/></clipPath>\n",
                i.stroke
            );
            let half = figure(deck, suit, rank, theme, false);
            let lower = figure(deck, suit, rank, theme, true);
            let _ = write!(
                body,
                "<g clip-path=\"url(#frame)\">{half}<g transform=\"rotate(180 100 150)\">{lower}</g></g>\n\
                 <path d=\"M44,150 H156\" stroke=\"{}\" stroke-width=\"2\"/>\n",
                i.stroke
            );
        }
    }
    svg(&body)
}
