pub const VERSION: &str = "0.0.1";

// -b  option; if 0, dmenu appears at bottom
pub const TOPBAR: bool = true;

// -fn option overrides fonts[0]; default X11 font or font set
pub const FONTS: &[&str] = &[
    "monospace:size=10",
    "NotoColorEmoji:pixelsize=8:antialias=true:autohint=true",
];
pub const PROMPT: &str = "";

pub const COLORS: &[[&str; 2]] = &[
    /*     fg         bg       */
    ["#bbbbbb", "#222222"], //SchemeNorm
    ["#eeeeee", "#005577"], //SchemeSel
    ["#000000", "#00ffff"], //SchemeOut
];

pub const ALPHA: u32 = 0xEE;
pub const OPAQUE: u32 = 0xFF;

pub const ALPHAS: &[[u32; 2]] = &[[OPAQUE, ALPHA], [OPAQUE, ALPHA], [OPAQUE, ALPHA]];

// -l option; if nonzero, dmenu uses vertical list with given number of lines
pub const LINES: u32 = 0;

//Characters not considered part of a word while deleting words
//for example: " /?\"&[]"
pub const WORD_DELIMETER: &str = " ";
