use std::{
    env,
    ffi::CStr,
    io::{BufRead, IsTerminal, stdin},
    process::{ExitCode, exit},
    ptr::NonNull,
    rc::Rc,
    thread::sleep,
    time::Duration,
};

use crate::{
    drw::{COLORS_PER_SCHEME, Drw},
    external::*,
};

mod config;
mod drw;
mod external;

const SCHEME_NORM: usize = 0;
const SCHEME_SEL: usize = 1;
const SCHEME_OUT: usize = 2;
const SCHEME_LAST: usize = 3;

#[inline(always)]
fn text_w(x: &str, drw: &mut Drw, lrpad: i32) -> u32 {
    drw.fontset_get_width(x) + lrpad as u32
}

#[inline(always)]
fn text_w_clamp(x: &str, n: u32, drw: &mut Drw, lrpad: i32) -> u32 {
    let w = drw.fontset_get_width_clamp(x, n) + lrpad as u32;
    w.min(n)
}

#[inline(always)]
fn intersect(x: i32, y: i32, w: i32, h: i32, r: *mut XineramaScreenInfo) -> i32 {
    i32::max(
        0,
        i32::min(
            x + w,
            unsafe { &*r }.x_org as i32 + unsafe { &*r }.width as i32,
        ) - i32::max(x, unsafe { &*r }.x_org as i32),
    ) * i32::max(
        0,
        i32::min(
            y + h,
            unsafe { &*r }.y_org as i32 + unsafe { &*r }.height as i32,
        ) - i32::max(y, unsafe { &*r }.y_org as i32),
    )
}

struct Globals {
    text: String,
    embed: String,
    bh: i32,
    mw: i32,
    mh: i32,
    inputw: i32,
    promptw: i32,
    passwd: bool,
    lrpad: i32,
    cursor: usize,
    items: Box<[Item]>, //Vec<Item>,
    /// Indices into `items` for the entries that currently match the query.
    matches: Vec<usize>,
    /// Reusable scratch buffers for `mtch()`, cleared (not reallocated) each call.
    lprefix: Vec<usize>,
    lsubstr: Vec<usize>,
    /// All four of these are positions into `matches` (never raw `items`
    /// indices), and are `None` exactly when `matches` is empty - mirroring
    /// the nullable `struct item *` pointers of the C original.
    curr: Option<usize>,
    next: Option<usize>,
    prev: Option<usize>,
    sel: Option<usize>,
    mon: i32,
    screen: i32,
    clip: Atom,
    utf8: Atom,
    dpy: NonNull<Display>,
    root: Window,
    parentwin: Window,
    win: Window,
    xic: XIC,
    drw: Drw,
    scheme: [Rc<[drw::Clr; 2]>; SCHEME_LAST],
    useargb: bool,
    visual: NonNull<Visual>,
    depth: i32,
    cmap: Colormap,
}

impl Globals {
    fn new(dpy: NonNull<Display>, drw: Drw) -> Self {
        // const EMPTY_SCHEME: [XftColor; drw::COLORS_PER_SCHEME] =
        // [XftColor::default(); drw::COLORS_PER_SCHEME];
        Self {
            text: String::new(),
            embed: String::new(),
            bh: 0,
            mw: 0,
            mh: 0,
            inputw: 0,
            promptw: 0,
            lrpad: 0,
            cursor: 0,
            items: Box::new([]),
            mon: -1,
            screen: 0,
            clip: 0,
            utf8: 0,
            dpy,
            root: 0,
            parentwin: 0,
            win: 0,
            xic: core::ptr::null_mut(),
            drw,
            scheme: (0..SCHEME_LAST)
                .map(|_| Rc::new([XftColor::default(); drw::COLORS_PER_SCHEME]))
                .collect::<Vec<_>>()
                .try_into()
                .expect("dimensions match"),
            matches: Vec::new(),
            lprefix: Vec::new(),
            lsubstr: Vec::new(),
            curr: None,
            sel: None,
            next: None,
            prev: None,
            useargb: false,
            visual: NonNull::dangling(),
            depth: 0,
            cmap: 0,
            passwd: false,
        }
    }
}

#[derive(Debug)]
struct Item {
    text: String,
    out: bool,
}

fn usage() {
    eprintln!("usage: dmenu [-bfivP] [-l lines] [-p prompt] [-fn font] [-m monitor]");
    eprintln!("             [-nb color] [-nf color] [-sb color] [-sf color] [-w windowid]");
    exit(1);
}

/// Mirrors C's `strncmp(s1, s2, n) == 0`: compares up to `n` bytes, but (like
/// a NUL-terminated C string) a string shorter than `n` only compares equal
/// to another string of the exact same (shorter) length. Never panics, even
/// when `n` exceeds either string's length.
fn strncmp(s1: &str, s2: &str, n: usize) -> bool {
    if s1.len() >= n && s2.len() >= n {
        s1.as_bytes()[..n] == s2.as_bytes()[..n]
    } else {
        s1 == s2
    }
}

/// Same as `strncmp`, but ASCII case-insensitive (matching C's locale-independent
/// `tolower()` byte folding rather than Rust's Unicode-aware `to_lowercase`,
/// which can change a string's byte length for some characters).
fn strncasecmp(s1: &str, s2: &str, n: usize) -> bool {
    fn ascii_lower_eq(a: &[u8], b: &[u8]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
    }
    if s1.len() >= n && s2.len() >= n {
        ascii_lower_eq(&s1.as_bytes()[..n], &s2.as_bytes()[..n])
    } else {
        ascii_lower_eq(s1.as_bytes(), s2.as_bytes())
    }
}

fn strstr<'a>(haystack: &'a str, needle: &str) -> Option<&'a str> {
    let pos = haystack.find(needle)?;
    Some(&haystack[pos..])
}

fn cistrstr<'a>(haystack: &'a str, needle: &str) -> Option<&'a str> {
    let pos = haystack.to_lowercase().find(&needle.to_lowercase())?;
    Some(&haystack[pos..])
}

struct Config {
    topbar: bool,
    fonts: Vec<String>,
    prompt: String,
    colors: Vec<[String; COLORS_PER_SCHEME]>,
    alphas: Vec<[u32; 2]>,
    lines: u32,
    word_delimeter: String,
    case_insensitive: bool,
}

impl<'a> Default for Config {
    fn default() -> Self {
        Self {
            topbar: config::TOPBAR,
            fonts: config::FONTS.iter().map(|s| s.to_string()).collect(),
            prompt: config::PROMPT.to_string(),
            colors: config::COLORS
                .iter()
                .map(|[a, b]| [a.to_string(), b.to_string()])
                .collect(),
            alphas: config::ALPHAS.iter().map(|a| *a).collect(),
            lines: config::LINES,
            word_delimeter: config::WORD_DELIMETER.to_string(),
            case_insensitive: false,
        }
    }
}

fn mtch(globals: &mut Globals, config: &Config) {
    // strtok(" ") collapses consecutive/leading/trailing delimiters and never
    // yields empty tokens; split(' ').filter(non-empty) mirrors that.
    let tokv: Vec<&str> = globals.text.split(' ').filter(|s| !s.is_empty()).collect();
    let len = tokv.first().map_or(0, |t| t.len());

    globals.matches.clear();
    globals.lprefix.clear();
    globals.lsubstr.clear();

    let fstrstr = if config.case_insensitive {
        cistrstr
    } else {
        strstr
    };
    let fstrncmp = if config.case_insensitive {
        strncasecmp
    } else {
        strncmp
    };

    for (k, item) in globals.items.iter().enumerate() {
        let mut i = 0;
        while i < tokv.len() {
            if fstrstr(&item.text, tokv[i]).is_none() {
                break;
            }
            i += 1
        }
        if i != tokv.len() {
            continue;
        }
        // exact matches go first, then prefixes, then substrings.
        // `fstrncmp` already returns true when the compared prefixes are
        // equal (unlike C's strncmp, which returns 0 for equal), so unlike
        // the C source these checks are NOT negated.
        if tokv.is_empty() || fstrncmp(&globals.text, &item.text, globals.text.len()) {
            globals.matches.push(k);
        } else if fstrncmp(tokv[0], &item.text, len) {
            globals.lprefix.push(k);
        } else {
            globals.lsubstr.push(k);
        }
    }

    if !globals.lprefix.is_empty() {
        globals.matches.append(&mut globals.lprefix);
    }
    if !globals.lsubstr.is_empty() {
        globals.matches.append(&mut globals.lsubstr);
    }

    globals.curr = (!globals.matches.is_empty()).then_some(0);
    globals.sel = globals.curr;
    calcoffsets(config, globals);
}

fn calcoffsets(config: &Config, globals: &mut Globals) {
    let n = if config.lines > 0 {
        config.lines as i32 * globals.bh
    } else {
        globals.mw
            - (globals.promptw
                + globals.inputw
                + text_w("<", &mut globals.drw, globals.lrpad) as i32
                + text_w(">", &mut globals.drw, globals.lrpad) as i32)
    };

    let mut i = 0;
    globals.next = globals.curr;
    while let Some(pos) = globals.next {
        let item = &globals.items[globals.matches[pos]];
        i += if config.lines > 0 {
            globals.bh
        } else {
            text_w_clamp(&item.text, n as u32, &mut globals.drw, globals.lrpad) as i32
        };
        if i > n {
            break;
        }
        globals.next = (pos + 1 < globals.matches.len()).then_some(pos + 1);
    }

    let mut i = 0;
    globals.prev = globals.curr;
    while let Some(pos) = globals.prev {
        if pos == 0 {
            break;
        }
        let item = &globals.items[globals.matches[pos - 1]];
        i += if config.lines > 0 {
            globals.bh
        } else {
            text_w_clamp(&item.text, n as u32, &mut globals.drw, globals.lrpad) as i32
        };
        if i > n {
            break;
        }
        globals.prev = Some(pos - 1);
    }
}

fn grabkeyboard(dpy: NonNull<Display>, globals: &Globals) {
    const GRAB_MODE_ASYNC: i32 = 1;
    const CURRENT_TIME: u64 = 0;
    const GRAB_SUCCESS: i32 = 0;

    if !globals.embed.is_empty() {
        return;
    }
    for _ in 0..1000 {
        if unsafe {
            XGrabKeyboard(
                dpy.as_ptr(),
                default_root_window(dpy.as_ptr()),
                1,
                GRAB_MODE_ASYNC,
                GRAB_MODE_ASYNC,
                CURRENT_TIME,
            )
        } == GRAB_SUCCESS
        {
            return;
        }
        sleep(Duration::from_nanos(1000000));
    }
    eprintln!("cannot grab keyboard");
    exit(1)
}

fn readstdin(config: &mut Config, globals: &mut Globals) {
    // Read raw bytes rather than `read_line` (which hard-requires valid UTF-8
    // for the *whole* line and aborts the entire read on the first invalid
    // line, silently discarding every subsequent line too). `from_utf8_lossy`
    // replaces invalid sequences with U+FFFD per-line instead, matching C's
    // tolerance for arbitrary bytes via `getline`.
    let mut buf = Vec::new();
    let mut stdin = stdin().lock();
    let mut items = Vec::new();
    if globals.passwd {
        globals.inputw = 0;
        config.lines = 0;
        return;
    }
    let mut tmpmax = 0;
    let mut imax = 0;
    let mut i = 0;
    loop {
        buf.clear();
        let len = stdin
            .read_until(b'\n', &mut buf)
            .expect("failed to read stdin");
        if len == 0 {
            break;
        }
        if buf.last() == Some(&b'\n') {
            buf.pop();
        }
        let text = String::from_utf8_lossy(&buf).into_owned();
        globals.drw.fonts[0].get_exts(&text, text.len() as u32, Some(&mut tmpmax), None);
        if tmpmax as i32 > globals.inputw {
            globals.inputw = tmpmax as i32;
            imax = i;
        }

        items.push(Item {
            text: text,
            out: false,
        });
        i += 1;
    }
    globals.items = items.into_boxed_slice();

    if !globals.items.is_empty() {
        globals.inputw = text_w(&globals.items[imax].text, &mut globals.drw, globals.lrpad) as i32;
    }
    config.lines = config.lines.min(globals.items.len() as u32);
}

fn setup(config: &mut Config, globals: &mut Globals) {
    let mut wa: XWindowAttributes = unsafe { core::mem::zeroed() };
    let mut swa: XSetWindowAttributes = unsafe { core::mem::zeroed() };
    let mut ch = XClassHint {
        res_name: c"dmenu".as_ptr().cast_mut(),
        res_class: c"dmenu".as_ptr().cast_mut(),
    };
    for j in 0..SCHEME_LAST {
        globals.scheme[j] = globals.drw.scm_create(&config.colors[j], &config.alphas[j]);
    }
    globals.clip = unsafe { XInternAtom(globals.dpy.as_ptr(), c"CLIPBOARD".as_ptr(), 0) };
    globals.utf8 = unsafe { XInternAtom(globals.dpy.as_ptr(), c"UTF8_STRING".as_ptr(), 0) };

    //Calculate menu geometry
    globals.bh = globals.drw.fonts[0].h as i32 + 2;
    config.lines = config.lines.max(0);
    globals.mh = (config.lines as i32 + 1) * globals.bh;

    let mut x = 0;
    let mut y = 0;
    let mut w: Window = 0;
    let mut dw: Window = 0;
    let mut dws: *mut Window = core::ptr::null_mut();
    let mut du = 0u32;

    #[cfg(feature = "xinerama")]
    {
        let mut test = globals.parentwin == globals.root;
        let mut info = core::ptr::null_mut();
        let mut n = 0;
        let mut di = 0;
        let mut i = 0;
        let mut area = 0;

        const POINTER_ROOT: u64 = 1;
        const NONE: u64 = 0;

        if test {
            info = unsafe { XineramaQueryScreens(globals.dpy.as_ptr(), &mut n) };
            test &= !info.is_null()
        }
        if test {
            unsafe { XGetInputFocus(globals.dpy.as_ptr(), &mut w, &mut di) };
            if globals.mon >= 0 && globals.mon < n {
                i = globals.mon;
            } else if w != globals.root && w != POINTER_ROOT && w != NONE {
                // find top-level window containing current input focus
                let mut pw;
                loop {
                    pw = w;
                    if unsafe {
                        XQueryTree(globals.dpy.as_ptr(), pw, &mut dw, &mut w, &mut dws, &mut du)
                    } != 0
                        && !dws.is_null()
                    {
                        unsafe { XFree(dws.cast()) };
                    }
                    if w == globals.root || w == pw {
                        break;
                    }
                }
                // find xinerama screen with which the window intersects most
                if unsafe { XGetWindowAttributes(globals.dpy.as_ptr(), pw, &mut wa) } != 0 {
                    for j in 0..n {
                        // for (j = 0; j < n; j++)
                        let a = intersect(wa.x, wa.y, wa.width, wa.height, unsafe {
                            info.add(j as usize)
                        });
                        if a > area {
                            area = a;
                            i = j;
                        }
                    }
                }
            }
            // no focused window is on screen, so use pointer location instead
            if globals.mon < 0
                && area == 0
                && unsafe {
                    XQueryPointer(
                        globals.dpy.as_ptr(),
                        globals.root,
                        &mut dw,
                        &mut dw,
                        &mut x,
                        &mut y,
                        &mut di,
                        &mut di,
                        &mut du,
                    )
                } != 0
            {
                for i in i..n {
                    if intersect(x, y, 1, 1, unsafe { info.add(i as usize) }) != 0 {
                        break;
                    }
                }
            }

            x = unsafe { &*info.add(i as usize) }.x_org as i32;
            y = unsafe { &*info.add(i as usize) }.y_org as i32
                + (if config.topbar {
                    0
                } else {
                    unsafe { &*info.add(i as usize) }.height as i32 - globals.mh
                });
            globals.mw = unsafe { &*info.add(i as usize) }.width as i32;
            unsafe { XFree(info.cast()) };
        } else {
            if unsafe { XGetWindowAttributes(globals.dpy.as_ptr(), globals.parentwin, &mut wa) }
                == 0
            {
                eprintln!(
                    "Could not get embedding window attributes: {:x}",
                    globals.parentwin
                );
                exit(1)
            }
            x = 0;
            y = if config.topbar {
                0
            } else {
                wa.height - globals.mh
            };
            globals.mw = wa.width;
        }
    }
    #[cfg(not(feature = "xinerama"))]
    {
        if unsafe { XGetWindowAttributes(globals.dpy.as_ptr(), globals.parentwin, &mut wa) } == 0 {
            eprintln!(
                "Could not get embedding window attributes: {:x}",
                globals.parentwin
            );
            exit(1)
        }
        x = 0;
        y = if config.topbar {
            0
        } else {
            wa.height - globals.mh
        };
        globals.mw = wa.width;
    }

    globals.promptw = if !config.prompt.is_empty() {
        text_w(&config.prompt, &mut globals.drw, globals.lrpad) as i32 - globals.lrpad / 4
    } else {
        0
    };
    globals.inputw = globals.inputw.min(globals.mw / 3); // input width: ~33% of monitor width
    mtch(globals, config);

    const EXPOSURE_MASK: i64 = 1 << 15;
    const KEY_PRESS_MASK: i64 = 1 << 0;
    const VISIBILITY_CHANGE_MASK: i64 = 1 << 16;
    const BUTTON_PRESS_MASK: i64 = 1 << 2;
    const COPY_FROM_PARENT: i64 = 0;
    const CW_OVERRIDE_REDIRECT: u64 = 1 << 9;
    const CW_BACK_PIXEL: u64 = 1 << 1;
    const CW_EVENT_MASK: u64 = 1 << 11;
    const CW_BORDER_PIXEL: u64 = 1 << 3;
    const CW_COLOR_MAP: u64 = 1 << 13;

    // create menu window
    swa.override_redirect = 1;
    swa.border_pixel = 0;
    swa.colormap = globals.cmap;
    swa.event_mask = EXPOSURE_MASK | KEY_PRESS_MASK | VISIBILITY_CHANGE_MASK | BUTTON_PRESS_MASK;

    globals.win = unsafe {
        XCreateWindow(
            globals.dpy.as_ptr(),
            globals.root,
            x,
            y,
            globals.mw as u32,
            globals.mh as u32,
            0,
            globals.depth,
            COPY_FROM_PARENT as u32,
            globals.visual.as_ptr(),
            CW_OVERRIDE_REDIRECT | CW_BACK_PIXEL | CW_BORDER_PIXEL | CW_COLOR_MAP | CW_EVENT_MASK,
            &mut swa,
        )
    };
    unsafe { XSetClassHint(globals.dpy.as_ptr(), globals.win, &mut ch) };

    // input methods
    let xim = unsafe {
        XOpenIM(
            globals.dpy.as_ptr(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };
    if xim.is_null() {
        eprintln!("XOpenIM failed: could not open input device");
        exit(1)
    }

    const XN_INPUT_STYLE: &CStr = c"inputStyle";
    const XIM_PREEDIT_NOTHING: i64 = 0x0008;
    const XIM_STATUS_NOTHING: i64 = 0x0400;
    const XN_CLIENT_WINDOW: &CStr = c"clientWindow";
    const XN_FOCUS_WINDOW: &CStr = c"focusWindow";

    globals.xic = unsafe {
        XCreateIC(
            xim,
            XN_INPUT_STYLE.as_ptr(),
            XIM_PREEDIT_NOTHING | XIM_STATUS_NOTHING,
            XN_CLIENT_WINDOW.as_ptr(),
            globals.win,
            XN_FOCUS_WINDOW.as_ptr(),
            globals.win,
            core::ptr::null_mut::<()>(),
        )
    };

    unsafe { XMapRaised(globals.dpy.as_ptr(), globals.win) };
    const FOCUS_CHANGE_MASK: i64 = 1 << 21;
    const SUBSTRUCTURE_NOTIFY_MASK: i64 = 1 << 19;
    if !globals.embed.is_empty() {
        unsafe { XReparentWindow(globals.dpy.as_ptr(), globals.win, globals.parentwin, x, y) };
        unsafe {
            XSelectInput(
                globals.dpy.as_ptr(),
                globals.parentwin,
                FOCUS_CHANGE_MASK | SUBSTRUCTURE_NOTIFY_MASK,
            )
        };
        if unsafe {
            XQueryTree(
                globals.dpy.as_ptr(),
                globals.parentwin,
                &mut dw,
                &mut w,
                &mut dws,
                &mut du,
            )
        } != 0
            && !dws.is_null()
        {
            for i in 0..du {
                if unsafe { *dws.add(i as usize) } == globals.win {
                    break;
                }
                unsafe {
                    XSelectInput(
                        globals.dpy.as_ptr(),
                        *dws.add(i as usize),
                        FOCUS_CHANGE_MASK,
                    )
                };
            }
            // for (i = 0; i < du && dws[i] != win; ++i)
            // dbg!("!!!");
            unsafe { XFree(dws.cast()) };
            // dbg!("!!!");
        }
        grabfocus(globals);
    }
    globals.drw.resize(globals.mw as u32, globals.mh as u32);

    drawmenu(globals, config);
}

fn drawmenu(globals: &mut Globals, config: &Config) {
    let mut x = 0;
    let mut y = 0;

    globals
        .drw
        .set_scheme(Rc::clone(&globals.scheme[SCHEME_NORM]));
    globals
        .drw
        .rect(0, 0, globals.mw as u32, globals.mh as u32, true, true);

    if !config.prompt.is_empty() {
        globals
            .drw
            .set_scheme(Rc::clone(&globals.scheme[SCHEME_SEL]));
        x = globals.drw.text(
            x,
            0,
            globals.promptw as u32,
            globals.bh as u32,
            globals.lrpad as u32 / 2,
            config.prompt.as_str(),
            0,
        );
    }

    let mut w = if config.lines > 0 || globals.matches.is_empty() {
        globals.mw - x
    } else {
        globals.inputw
    };
    globals
        .drw
        .set_scheme(Rc::clone(&globals.scheme[SCHEME_NORM]));
    if globals.passwd {
        let censort = '.'.to_string().repeat(globals.text.len());
        globals.drw.text(
            x,
            0,
            w as u32,
            globals.bh as u32,
            globals.lrpad as u32 / 2,
            &censort,
            0,
        );
    } else {
        globals.drw.text(
            x,
            0,
            w as u32,
            globals.bh as u32,
            globals.lrpad as u32 / 2,
            globals.text.as_str(),
            0,
        );
    }

    let mut curpos = text_w(&globals.text, &mut globals.drw, globals.lrpad)
        - text_w(
            &globals.text[globals.cursor..],
            &mut globals.drw,
            globals.lrpad,
        );
    curpos += globals.lrpad as u32 / 2 - 1;
    if curpos < w as u32 {
        globals
            .drw
            .set_scheme(Rc::clone(&globals.scheme[SCHEME_NORM]));
        globals
            .drw
            .rect(x + curpos as i32, 2, 2, globals.bh as u32 - 4, true, false);
    }

    if config.lines > 0 {
        // draw vertical list: walk matches[curr..next]
        let mut pos = globals.curr;
        while pos != globals.next {
            let Some(p) = pos else { break };
            y += globals.bh;
            Item::draw(globals.matches[p], Some(p), x, y, globals.mw - x, globals);
            pos = (p + 1 < globals.matches.len()).then_some(p + 1);
        }
    } else if !globals.matches.is_empty() {
        // draw horizontal list
        x += globals.inputw;
        w = text_w("<", &mut globals.drw, globals.lrpad) as i32;
        if globals.curr.is_some_and(|p| p > 0) {
            globals
                .drw
                .set_scheme(Rc::clone(&globals.scheme[SCHEME_NORM]));
            globals.drw.text(
                x,
                0,
                w as u32,
                globals.bh as u32,
                globals.lrpad as u32 / 2,
                "<",
                0,
            );
        }
        x += w;
        let mut pos = globals.curr;
        while pos != globals.next {
            let Some(p) = pos else { break };
            let item_idx = globals.matches[p];
            x = Item::draw(
                item_idx,
                Some(p),
                x,
                0,
                text_w_clamp(
                    &globals.items[item_idx].text,
                    globals.mw as u32 - x as u32 - text_w(">", &mut globals.drw, globals.lrpad),
                    &mut globals.drw,
                    globals.lrpad,
                ) as i32,
                globals,
            );
            pos = (p + 1 < globals.matches.len()).then_some(p + 1);
        }
        if globals.next.is_some() {
            w = text_w(">", &mut globals.drw, globals.lrpad) as i32;
            globals
                .drw
                .set_scheme(Rc::clone(&globals.scheme[SCHEME_NORM]));
            globals.drw.text(
                globals.mw - w,
                0,
                w as u32,
                globals.bh as u32,
                globals.lrpad as u32 / 2,
                ">",
                0,
            );
        }
    }

    globals
        .drw
        .map(globals.win, 0, 0, globals.mw as u32, globals.mh as u32);
}

impl Item {
    fn draw(
        item_idx: usize,
        pos: Option<usize>,
        x: i32,
        y: i32,
        w: i32,
        globals: &mut Globals,
    ) -> i32 {
        if pos == globals.sel {
            globals
                .drw
                .set_scheme(Rc::clone(&globals.scheme[SCHEME_SEL]));
        } else if globals.items[item_idx].out {
            globals
                .drw
                .set_scheme(Rc::clone(&globals.scheme[SCHEME_OUT]));
        } else {
            globals
                .drw
                .set_scheme(Rc::clone(&globals.scheme[SCHEME_NORM]));
        }

        globals.drw.text(
            x,
            y,
            w as u32,
            globals.bh as u32,
            globals.lrpad as u32 / 2,
            globals.items[item_idx].text.as_str(),
            0,
        )
    }
}

fn grabfocus(globals: &Globals) {
    let mut focuswin: Window = 0;
    let mut revertwin = 0;
    const CURRENT_TIME: u64 = 0;
    const REVERT_TO_PARENT: i32 = 2;
    for _ in 0..100 {
        unsafe { XGetInputFocus(globals.dpy.as_ptr(), &mut focuswin, &mut revertwin) };
        if focuswin == globals.win {
            return;
        }
        unsafe {
            XSetInputFocus(
                globals.dpy.as_ptr(),
                globals.win,
                REVERT_TO_PARENT,
                CURRENT_TIME,
            )
        };
        sleep(Duration::from_nanos(10000000));
    }
    eprintln!("cannot grasp focus");
    exit(1)
}

/// Runs the X event loop until something requests the program end, and
/// returns the process exit code. `main()` is responsible for cleanup once
/// this returns (see `cleanup()`), matching C's `cleanup(); exit(n);` pattern
/// but without skipping Rust's destructors.
fn run(globals: &mut Globals, config: &Config) -> i32 {
    let mut ev: XEvent = unsafe { core::mem::zeroed() };

    const DESTROY_NOTIFY: i32 = 17;
    const EXPOSE: i32 = 12;
    const FOCUS_IN: i32 = 9;
    const KEY_PRESS: i32 = 2;
    const BUTTON_PRESS: i32 = 4;
    const SELECTION_NOTIFY: i32 = 31;
    const VISIBILITY_NOTIFY: i32 = 15;
    const VISIBILITY_UNOBSCURED: i32 = 0;

    // XNextEvent blocks for the next event and always returns 0; C's
    // `while (!XNextEvent(...))` loops forever until a handler below ends
    // the program (a previous `!= 0` here made the loop body unreachable).
    while unsafe { XNextEvent(globals.dpy.as_ptr(), &mut ev) } == 0 {
        if unsafe { XFilterEvent(&mut ev, globals.win) } != 0 {
            continue;
        }

        match unsafe { ev.typ } {
            DESTROY_NOTIFY => {
                if unsafe { ev.xdestroywindow.window } != globals.win {
                    // Not our window (e.g. a sibling under an embedding
                    // parent) - ignore this event and keep running, matching
                    // C's switch-`break` (a previous `break` here escaped the
                    // whole event loop instead of just this event).
                    continue;
                }
                return 1;
            }
            BUTTON_PRESS => {
                if let Some(code) = button_press(unsafe { &mut ev.xbutton }, globals, config) {
                    return code;
                }
            }
            EXPOSE => {
                if unsafe { ev.xexpose.count } == 0 {
                    globals
                        .drw
                        .map(globals.win, 0, 0, globals.mw as u32, globals.mh as u32);
                }
            }
            FOCUS_IN => {
                if unsafe { ev.xfocus.window } != globals.win {
                    grabfocus(globals);
                }
            }
            KEY_PRESS => {
                if let Some(code) = keypress(unsafe { &mut ev.xkey }, globals, config) {
                    return code;
                }
            }
            SELECTION_NOTIFY => {
                if unsafe { ev.xselection.property } == globals.utf8 {
                    paste(globals, config)
                }
            }
            VISIBILITY_NOTIFY => {
                if unsafe { ev.xvisibility.state } != VISIBILITY_UNOBSCURED {
                    unsafe { XRaiseWindow(globals.dpy.as_ptr(), globals.win) };
                }
            }
            _ => {}
        }
    }
    1
}

/// True if `sel` is currently positioned one step above the top of the
/// visible page and can move up; moves it and rescrolls the page when needed
/// -- mirroring C's `if (sel && sel->left && (sel = sel->left)->right == curr)`,
/// which unconditionally moves `sel` back one (as a side effect of evaluating
/// the `&&` chain) and only *additionally* rescrolls if that move crossed the
/// page-top boundary.
fn sel_move_up(globals: &mut Globals, config: &Config) {
    if let Some(sel_idx) = globals.sel {
        if sel_idx > 0 {
            let crossed_page_top = globals.curr == Some(sel_idx);
            globals.sel = Some(sel_idx - 1);
            if crossed_page_top {
                globals.curr = globals.prev;
                calcoffsets(config, globals);
            }
        }
    }
}

/// Symmetric counterpart of `sel_move_up`, mirroring
/// `if (sel && sel->right && (sel = sel->right) == next)`.
fn sel_move_down(globals: &mut Globals, config: &Config) {
    if let Some(sel_idx) = globals.sel {
        if sel_idx + 1 < globals.matches.len() {
            let new_sel = sel_idx + 1;
            globals.sel = Some(new_sel);
            if globals.next == Some(new_sel) {
                globals.curr = globals.next;
                calcoffsets(config, globals);
            }
        }
    }
}

/// Matches C's `strchr(worddelimiters, c)` (byte-wise, not rune-aware -
/// consistent with the C original, which also inspects individual bytes).
fn is_word_delim(b: u8, config: &Config) -> bool {
    config.word_delimeter.as_bytes().contains(&b)
}

fn button_press(ev: &mut XButtonEvent, globals: &mut Globals, config: &Config) -> Option<i32> {
    let mut x = 0;
    let mut y = 0;
    let h = globals.bh;

    if ev.window != globals.win {
        return None;
    }

    //Right-click: exit
    const BUTTON_1: u32 = 1;
    const BUTTON_2: u32 = 2;
    const BUTTON_3: u32 = 3;
    const BUTTON_4: u32 = 4;
    const BUTTON_5: u32 = 5;
    if ev.button == BUTTON_3 {
        return Some(1);
    }

    // Caps Lock and Num Lock are reported in the event state alongside the
    // "real" modifiers, but should not affect click handling - strip them so
    // e.g. a plain click doesn't get misread as a modified one just because
    // Num Lock happens to be on.
    const LOCK_MASK: u32 = 1 << 1;
    const NUM_LOCK_MASK: u32 = 1 << 4;
    let state = ev.state & !(LOCK_MASK | NUM_LOCK_MASK);

    if !config.prompt.is_empty() {
        x += globals.promptw;
    }

    // input field
    let mut w = if config.lines > 0 || globals.matches.is_empty() {
        globals.mw - x
    } else {
        globals.inputw
    };

    // left-click on input: clear input,
    // NOTE: if there is no left-arrow the space for < is reserved so
    //       add that to the input width
    if ev.button == BUTTON_1
        && ((config.lines <= 0
            && ev.x >= 0
            && ev.x
                <= x + w
                    + (if globals.prev.is_none() || !globals.curr.is_some_and(|p| p > 0) {
                        text_w("<", &mut globals.drw, globals.lrpad) as i32
                    } else {
                        0
                    }))
            || (config.lines > 0 && ev.y >= y && ev.y <= y + h))
    {
        // insert_text("", globals, config);
        globals.text.clear();
        globals.cursor = 0;
        mtch(globals, config);
        drawmenu(globals, config);
        return None;
    }

    const SHIFT_MASK: u32 = 1 << 0;
    const CONTROL_MASK: u32 = 1 << 2;
    const CURRENT_TIME: u64 = 0;
    const XA_PRIMARY: Atom = 1;
    // middle-mouse click: paste selection
    if ev.button == BUTTON_2 {
        unsafe {
            XConvertSelection(
                globals.dpy.as_ptr(),
                if state & SHIFT_MASK != 0 {
                    globals.clip
                } else {
                    XA_PRIMARY
                },
                globals.utf8,
                globals.utf8,
                globals.win,
                CURRENT_TIME,
            )
        };
        drawmenu(globals, config);
        return None;
    }

    /* scroll up */
    if ev.button == BUTTON_4 && globals.prev.is_some() {
        globals.curr = globals.prev;
        globals.sel = globals.curr;
        calcoffsets(config, globals);
        drawmenu(globals, config);
        return None;
    }
    /* scroll down */
    if ev.button == BUTTON_5 && globals.next.is_some() {
        globals.curr = globals.next;
        globals.sel = globals.curr;
        calcoffsets(config, globals);
        drawmenu(globals, config);
        return None;
    }

    if ev.button != BUTTON_1 {
        return None;
    }
    if state & !CONTROL_MASK != 0 {
        return None;
    }
    if config.lines > 0 {
        /* vertical list: (ctrl)left-click on item */
        // w = globals.mw - x;
        let Some(mut item) = globals.curr else {
            return None;
        };
        let n = globals.next.unwrap_or(globals.matches.len());
        while item < n {
            // for (item = curr; item != next; item = item->right) {
            y += h;
            if ev.y >= y && ev.y <= (y + h) {
                println!("{}", globals.items[globals.matches[item]].text);
                // puts(item->text);
                if (state & CONTROL_MASK) == 0 {
                    return Some(0);
                    // exit(0);
                }
                globals.sel = Some(item);
                if let Some(sel) = globals.sel {
                    globals.items[globals.matches[sel]].out = true;
                    drawmenu(globals, config);
                }
                return None;
            }
            item += 1
        }
    } else if !globals.matches.is_empty() {
        /* left-click on left arrow */
        x += globals.inputw;
        w = text_w("<", &mut globals.drw, globals.lrpad) as i32;
        if globals.prev.is_some()
            && let Some(c) = globals.curr
            && c > 0
        {
            if ev.x >= x && ev.x <= x + w {
                globals.curr = globals.prev;
                globals.sel = globals.curr;
                calcoffsets(config, globals);
                drawmenu(globals, config);
                return None;
            }
        }
        /* horizontal list: (ctrl)left-click on item */
        let Some(mut item) = globals.curr else {
            return None;
        };
        let n = globals.next.unwrap_or(globals.matches.len());
        while item < n {
            // for (item = curr; item != next; item = item->right) {
            x += w;
            w = i32::min(
                text_w(
                    &globals.items[globals.matches[item]].text,
                    &mut globals.drw,
                    globals.lrpad,
                ) as i32,
                globals.mw - x - text_w(">", &mut globals.drw, globals.lrpad) as i32,
            );
            if ev.x >= x && ev.x <= x + w {
                println!("{}", globals.items[globals.matches[item]].text);
                // puts(item->text);
                if state & CONTROL_MASK == 0 {
                    return Some(0);
                }
                globals.sel = Some(item);
                if let Some(sel) = globals.sel {
                    globals.items[globals.matches[sel]].out = true;
                    drawmenu(globals, config);
                }
                return None;
            }
            item += 1;
        }
        /* left-click on right arrow */
        w = text_w(">", &mut globals.drw, globals.lrpad) as i32;
        x = globals.mw - w;
        if globals.next.is_some() && ev.x >= x && ev.x <= x + w {
            globals.curr = globals.next;
            globals.sel = globals.curr;
            calcoffsets(config, globals);
            drawmenu(globals, config);
            return None;
        }
    }
    None
}

fn keypress(ev: &mut XKeyEvent, globals: &mut Globals, config: &Config) -> Option<i32> {
    const NO_SYMBOL: u64 = 0;

    let mut buf = [0i8; 64];
    let mut ksym: KeySym = NO_SYMBOL;
    let mut status: Status = 0;
    let len = unsafe {
        XmbLookupString(
            globals.xic,
            ev,
            buf.as_mut_ptr(),
            buf.len() as i32,
            &mut ksym,
            &mut status,
        )
    };

    let mut skip = false;
    const X_LOOKUP_CHARS: i32 = 2;
    const X_LOOKUP_KEYSYM: i32 = 3;
    const X_LOOKUP_BOTH: i32 = 4;
    match status {
        X_LOOKUP_CHARS => skip = true,
        X_LOOKUP_KEYSYM | X_LOOKUP_BOTH => {}
        _ => return None,
    }

    const SHIFT_MASK: u32 = 1 << 0;
    const CONTROL_MASK: u32 = 1 << 2;
    const MOD1_MASK: u32 = 1 << 3;

    if !skip {
        if ev.state & CONTROL_MASK != 0 {
            match ksym {
                keycodes::XK_a => ksym = keycodes::XK_Home,
                keycodes::XK_b => ksym = keycodes::XK_Left,
                keycodes::XK_c => ksym = keycodes::XK_Escape,
                keycodes::XK_d => ksym = keycodes::XK_Delete,
                keycodes::XK_e => ksym = keycodes::XK_End,
                keycodes::XK_f => ksym = keycodes::XK_Right,
                keycodes::XK_g => ksym = keycodes::XK_Escape,
                keycodes::XK_h => ksym = keycodes::XK_BackSpace,
                keycodes::XK_i => ksym = keycodes::XK_Tab,
                keycodes::XK_j | keycodes::XK_J | keycodes::XK_m | keycodes::XK_M => {
                    ksym = keycodes::XK_Return;
                    ev.state &= !CONTROL_MASK;
                }
                keycodes::XK_n => ksym = keycodes::XK_Down,
                keycodes::XK_p => ksym = keycodes::XK_Up,
                keycodes::XK_k => {
                    // delete right
                    globals.text.truncate(globals.cursor);
                    mtch(globals, config);
                }
                keycodes::XK_u => {
                    // delete left
                    delete_before_cursor(globals.cursor, globals, config);
                }
                keycodes::XK_w => {
                    // delete word
                    while globals.cursor > 0
                        && is_word_delim(globals.text.as_bytes()[next_rune(-1, globals)], config)
                    {
                        let del = globals.cursor - next_rune(-1, globals);
                        delete_before_cursor(del, globals, config);
                    }
                    while globals.cursor > 0
                        && !is_word_delim(globals.text.as_bytes()[next_rune(-1, globals)], config)
                    {
                        let del = globals.cursor - next_rune(-1, globals);
                        delete_before_cursor(del, globals, config);
                    }
                }
                keycodes::XK_y | keycodes::XK_Y => {
                    // paste selection
                    const SHIFT_MASK: u32 = 1 << 0;
                    const XA_PRIMARY: Atom = 1;
                    const CURRENT_TIME: u64 = 0;
                    unsafe {
                        XConvertSelection(
                            globals.dpy.as_ptr(),
                            if (ev.state & SHIFT_MASK) != 0 {
                                globals.clip
                            } else {
                                XA_PRIMARY
                            },
                            globals.utf8,
                            globals.utf8,
                            globals.win,
                            CURRENT_TIME,
                        )
                    };
                    return None;
                }
                keycodes::XK_Left | keycodes::XK_KP_Left => {
                    move_word_edge(-1, globals, config);
                    drawmenu(globals, config);
                    return None;
                }
                keycodes::XK_Right | keycodes::XK_KP_Right => {
                    move_word_edge(1, globals, config);
                    drawmenu(globals, config);
                    return None;
                }
                keycodes::XK_Return | keycodes::XK_KP_Enter => {}
                keycodes::XK_bracketleft => {
                    return Some(1);
                }
                _ => return None,
            }
        } else if ev.state & MOD1_MASK != 0 {
            match ksym {
                keycodes::XK_b => {
                    move_word_edge(-1, globals, config);
                    drawmenu(globals, config);
                    return None;
                }
                keycodes::XK_f => {
                    move_word_edge(1, globals, config);
                    drawmenu(globals, config);
                    return None;
                }
                keycodes::XK_g => ksym = keycodes::XK_Home,
                keycodes::XK_G => ksym = keycodes::XK_End,
                keycodes::XK_h => ksym = keycodes::XK_Up,
                keycodes::XK_j => ksym = keycodes::XK_Next,
                keycodes::XK_k => ksym = keycodes::XK_Prior,
                keycodes::XK_l => ksym = keycodes::XK_Down,
                _ => return None,
            }
        }
    }

    match (skip, ksym) {
        (false, keycodes::XK_Delete) | (false, keycodes::XK_KP_Delete) => {
            if globals.cursor >= globals.text.len() {
                return None;
            }
            globals.cursor = next_rune(1, globals);
            if globals.cursor == 0 {
                return None;
            }
            let del = globals.cursor - next_rune(-1, globals);
            delete_before_cursor(del, globals, config);
        }
        (false, keycodes::XK_BackSpace) => {
            if globals.cursor == 0 {
                return None;
            }
            let del = globals.cursor - next_rune(-1, globals);
            delete_before_cursor(del, globals, config);
        }
        (false, keycodes::XK_End) | (false, keycodes::XK_KP_End) => {
            if globals.cursor < globals.text.len() {
                globals.cursor = globals.text.len();
                drawmenu(globals, config);
                return None;
            }
            if globals.next.is_some() {
                // jump to end of list and position items in reverse
                globals.curr = Some(globals.matches.len() - 1); // matchend
                calcoffsets(config, globals);
                globals.curr = globals.prev;
                calcoffsets(config, globals);
                while globals.next.is_some() {
                    let Some(p) = globals.curr else { break };
                    if p + 1 >= globals.matches.len() {
                        break;
                    }
                    globals.curr = Some(p + 1);
                    calcoffsets(config, globals);
                }
            }
            globals.sel = (!globals.matches.is_empty()).then_some(globals.matches.len() - 1);
        }
        (false, keycodes::XK_Escape) => {
            return Some(1);
        }
        (false, keycodes::XK_Home) | (false, keycodes::XK_KP_Home) => {
            if globals.sel.is_none() || globals.sel == Some(0) {
                globals.cursor = 0;
                drawmenu(globals, config);
                return None;
            }
            globals.curr = Some(0);
            globals.sel = globals.curr;
            calcoffsets(config, globals);
        }
        (false, keycodes::XK_Left) | (false, keycodes::XK_KP_Left) => {
            if globals.cursor > 0
                && (globals.sel.is_none() || globals.sel == Some(0) || config.lines > 0)
            {
                globals.cursor = next_rune(-1, globals);
                drawmenu(globals, config);
                return None;
            }
            if config.lines > 0 {
                return None;
            }
            sel_move_up(globals, config);
        }
        (false, keycodes::XK_Up) | (false, keycodes::XK_KP_Up) => {
            sel_move_up(globals, config);
        }
        (false, keycodes::XK_Next) | (false, keycodes::XK_KP_Next) => {
            let next_pos = globals.next?;
            globals.curr = Some(next_pos);
            globals.sel = globals.curr;
            calcoffsets(config, globals);
        }
        (false, keycodes::XK_Prior) | (false, keycodes::XK_KP_Prior) => {
            let prev_pos = globals.prev?;
            globals.curr = Some(prev_pos);
            globals.sel = globals.curr;
            calcoffsets(config, globals);
        }
        (false, keycodes::XK_Return) | (false, keycodes::XK_KP_Enter) => {
            match globals.sel.filter(|_| ev.state & SHIFT_MASK == 0) {
                Some(sel_idx) => println!("{}", globals.items[globals.matches[sel_idx]].text),
                None => println!("{}", globals.text),
            }
            if ev.state & CONTROL_MASK == 0 {
                return Some(0);
            }
            if let Some(sel_idx) = globals.sel {
                globals.items[globals.matches[sel_idx]].out = true;
            }
        }
        (false, keycodes::XK_Right) | (false, keycodes::XK_KP_Right) => {
            if globals.cursor < globals.text.len() {
                globals.cursor = next_rune(1, globals);
                drawmenu(globals, config);
                return None;
            }
            if config.lines > 0 {
                return None;
            }
            sel_move_down(globals, config);
        }
        (false, keycodes::XK_Down) | (false, keycodes::XK_KP_Down) => {
            sel_move_down(globals, config);
        }
        (false, keycodes::XK_Tab) => {
            let sel_idx = globals.sel?;
            globals.text = globals.items[globals.matches[sel_idx]].text.clone();
            globals.cursor = globals.text.len();
            mtch(globals, config);
        }

        (true, _) | (false, _) => {
            // Matches C's `if (!iscntrl((unsigned char)*buf)) insert(buf, len);`:
            // only the first byte is inspected (a real control character is
            // always a single ASCII byte, never the leading byte of a valid
            // multi-byte UTF-8 sequence XmbLookupString would otherwise hand
            // back), so this correctly accepts ordinary typed characters.
            if !(buf[0] as u8 as char).is_control() {
                let bytes =
                    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, len as usize) };
                if let Ok(s) = str::from_utf8(bytes) {
                    insert_text(s, globals, config);
                }
            }
        }
    }

    drawmenu(globals, config);
    None
}

fn next_rune(inc: isize, globals: &Globals) -> usize {
    let bytes = globals.text.as_bytes();
    let mut n = globals.cursor as isize + inc;
    while n + inc >= 0 && (n as usize) < bytes.len() && bytes[n as usize] & 0xc0 == 0x80 {
        n += inc;
    }
    n.max(0) as usize
}

fn move_word_edge(dir: i32, globals: &mut Globals, config: &Config) {
    if dir < 0 {
        // move cursor to the start of the word
        while globals.cursor > 0
            && is_word_delim(globals.text.as_bytes()[next_rune(-1, globals)], config)
        {
            globals.cursor = next_rune(-1, globals);
        }
        while globals.cursor > 0
            && !is_word_delim(globals.text.as_bytes()[next_rune(-1, globals)], config)
        {
            globals.cursor = next_rune(-1, globals);
        }
    } else {
        // move cursor to the end of the word
        while globals.cursor < globals.text.len()
            && is_word_delim(globals.text.as_bytes()[globals.cursor], config)
        {
            globals.cursor = next_rune(1, globals);
        }
        while globals.cursor < globals.text.len()
            && !is_word_delim(globals.text.as_bytes()[globals.cursor], config)
        {
            globals.cursor = next_rune(1, globals);
        }
    }
}

/// Consumes `globals`, releasing its X resources (Pixmap/GC/font cache, via
/// `Drop for Drw`/`Fnt`) *before* closing the display connection - mirroring
/// C's `cleanup()`, which calls `drw_free()` ahead of `XSync`/`XCloseDisplay`.
/// Every exit path used to call this with a borrow and then immediately call
/// `std::process::exit()`, which skips destructors entirely and leaked the
/// Pixmap/GC/font cache on every normal exit; now `main()` calls this once,
/// after `run()` returns, and lets the process exit normally afterward.
fn cleanup(globals: Globals) {
    const CURRENT_TIME: u64 = 0;
    unsafe { XUngrabKeyboard(globals.dpy.as_ptr(), CURRENT_TIME) };
    let dpy = globals.dpy;
    drop(globals);
    unsafe {
        XSync(dpy.as_ptr(), 0);
        XCloseDisplay(dpy.as_ptr());
    }
}

fn paste(globals: &mut Globals, config: &Config) {
    let mut p: *mut i8 = core::ptr::null_mut();
    let mut di = 0;
    let mut dl = 0u64;
    let mut da: Atom = 0;
    const SUCCESS: i32 = 0;
    // Matches C's `sizeof text` (BUFSIZ, 8192) used as the max property size
    // to request; unrelated to the now-dynamically-sized `globals.text`.
    const TEXT_BUF_HINT: i64 = 8192;

    if unsafe {
        XGetWindowProperty(
            globals.dpy.as_ptr(),
            globals.win,
            globals.utf8,
            0,
            TEXT_BUF_HINT / 4 + 1,
            0,
            globals.utf8,
            &mut da,
            &mut di,
            &mut dl,
            &mut dl,
            (&mut p) as *mut *mut i8,
        )
    } == SUCCESS
        && !p.is_null()
    {
        let full = unsafe { CStr::from_ptr(p) }.to_bytes();
        let paste_bytes = match full.iter().position(|&b| b == b'\n') {
            Some(pos) => &full[..pos],
            None => full,
        };
        insert_text(&String::from_utf8_lossy(paste_bytes), globals, config);
        unsafe { XFree(p.cast()) };
    }
    drawmenu(globals, config);
}

/// Inserts `s` at the cursor and advances the cursor past it.
fn insert_text(s: &str, globals: &mut Globals, config: &Config) {
    if s.is_empty() {
        return;
    }
    globals.text.insert_str(globals.cursor, s);
    globals.cursor += s.len();
    mtch(globals, config);
}

/// Deletes the `byte_len` bytes immediately before the cursor.
fn delete_before_cursor(byte_len: usize, globals: &mut Globals, config: &Config) {
    let start = globals.cursor.saturating_sub(byte_len);
    globals.text.replace_range(start..globals.cursor, "");
    globals.cursor = start;
    mtch(globals, config);
}

fn read_xresources(config: &mut Config) {
    unsafe { XrmInitialize() };

    let display = unsafe { XOpenDisplay(core::ptr::null_mut()) };
    let xrm = unsafe { XResourceManagerString(display) };
    if !xrm.is_null() {
        let mut typ: *mut i8 = core::ptr::null_mut();
        let xdb = unsafe { XrmGetStringDatabase(xrm) };
        let mut xval: XrmValue = unsafe { core::mem::zeroed() };

        if unsafe {
            XrmGetResource(
                xdb,
                c"dmenu.font".as_ptr(),
                c"*".as_ptr(),
                &mut typ,
                &mut xval,
            )
        } != 0
        {
            /* font or font set */
            config.fonts[0] = unsafe {
                CStr::from_ptr(xval.addr)
                    .to_str()
                    .expect("valid str")
                    .to_owned()
            };
        }
        if unsafe {
            XrmGetResource(
                xdb,
                c"dmenu.color0".as_ptr(),
                c"*".as_ptr(),
                &mut typ,
                &mut xval,
            )
        } != 0
        {
            /* normal background color */
            config.colors[SCHEME_NORM][drw::COL_BG] = unsafe {
                CStr::from_ptr(xval.addr)
                    .to_str()
                    .expect("valid str")
                    .to_owned()
            }
        }
        if unsafe {
            XrmGetResource(
                xdb,
                c"dmenu.color4".as_ptr(),
                c"*".as_ptr(),
                &mut typ,
                &mut xval,
            )
        } != 0
        {
            /* normal foreground color */
            config.colors[SCHEME_NORM][drw::COL_FG] = unsafe {
                CStr::from_ptr(xval.addr)
                    .to_str()
                    .expect("valid str")
                    .to_owned()
            }
        }
        if unsafe {
            XrmGetResource(
                xdb,
                c"dmenu.color4".as_ptr(),
                c"*".as_ptr(),
                &mut typ,
                &mut xval,
            )
        } != 0
        {
            /* selected background color */
            config.colors[SCHEME_SEL][drw::COL_BG] = unsafe {
                CStr::from_ptr(xval.addr)
                    .to_str()
                    .expect("valid str")
                    .to_owned()
            }
        }
        if unsafe {
            XrmGetResource(
                xdb,
                c"dmenu.color0".as_ptr(),
                c"*".as_ptr(),
                &mut typ,
                &mut xval,
            )
        } != 0
        {
            /* selected foreground color */
            config.colors[SCHEME_SEL][drw::COL_FG] = unsafe {
                CStr::from_ptr(xval.addr)
                    .to_str()
                    .expect("valid str")
                    .to_owned()
            }
        }

        unsafe { XrmDestroyDatabase(xdb) };
    }
    unsafe { XCloseDisplay(display) };
}

fn xinitvisual(dpy: NonNull<Display>, screen: i32, root: u64) -> (NonNull<Visual>, i32, u64, bool) {
    const TRUE_COLOR: i32 = 4;
    let infos: *mut XVisualInfo;
    let mut fmt: *mut XRenderPictFormat;

    let mut tpl: XVisualInfo = unsafe { core::mem::zeroed() };
    tpl.screen = screen;
    tpl.depth = 32;
    tpl.class = TRUE_COLOR;

    const VISUAL_SCREEN_MASK: i64 = 0x2;
    const VISUAL_DEPTH_MASK: i64 = 0x4;
    const VISUAL_CLASS_MASK: i64 = 0x8;
    const PICT_TYPE_DIRECT: i32 = 1;
    const ALLOC_NONE: i32 = 0;

    let masks = VISUAL_SCREEN_MASK | VISUAL_DEPTH_MASK | VISUAL_CLASS_MASK;
    let mut visual: *mut Visual = core::ptr::null_mut();
    let mut depth = 0;
    let mut cmap = 0;
    let mut nitems = 0;
    let mut useargb = false;
    infos = unsafe { XGetVisualInfo(dpy.as_ptr(), masks, &mut tpl, &mut nitems) };

    for i in 0..nitems {
        fmt = unsafe { XRenderFindVisualFormat(dpy.as_ptr(), { &*infos.add(i as usize) }.visual) };
        if unsafe { &*fmt }.typ == PICT_TYPE_DIRECT && unsafe { &*fmt }.direct.alpha_mask != 0 {
            visual = unsafe { &*infos.add(i as usize) }.visual;
            depth = unsafe { &*infos.add(i as usize) }.depth;
            cmap = unsafe { XCreateColormap(dpy.as_ptr(), root, visual, ALLOC_NONE) };
            useargb = true;
            break;
        }
    }

    unsafe { XFree(infos.cast()) };

    if visual.is_null() {
        visual = unsafe { default_visual(dpy.as_ptr(), screen) };
        depth = unsafe { default_depth(dpy.as_ptr(), screen) };
        cmap = unsafe { default_colormap(dpy.as_ptr(), screen) };
    }

    (
        NonNull::new(visual).expect("valid either by construction or by default visual"),
        depth,
        cmap,
        useargb,
    )
}

fn main() -> ExitCode {
    let mut config = Config::default();

    let mut fast = false;
    let mut mon = -1;
    let mut embed = String::new();
    let mut passwd = false;

    let args: Vec<String> = env::args().collect();
    let mut i = 1;

    read_xresources(&mut config);

    while i < args.len() {
        if args[i] == "-v" {
            eprintln!("demenu-{}", config::VERSION);
            return ExitCode::SUCCESS;
        } else if args[i] == "-b" {
            config.topbar = false
        } else if args[i] == "-f" {
            fast = true
        } else if args[i] == "-i" {
            config.case_insensitive = true;
        } else if args[i] == "-P" {
            passwd = true;
        } else if i + 1 == args.len() {
            usage();
        }
        // these options take one argument
        else if args[i] == "-l" {
            i += 1;
            config.lines = args[i].parse().expect("invalid value for lines variable");
        } else if args[i] == "-m" {
            i += 1;
            mon = args[i].parse().expect("invalid value for lines variable");
        } else if args[i] == "-p" {
            i += 1;
            config.prompt = args[i].clone();
        } else if args[i] == "-fn" {
            i += 1;
            config.fonts[0] = args[i].clone();
        } else if args[i] == "-nb" {
            i += 1;
            config.colors[SCHEME_NORM][drw::COL_BG] = args[i].clone();
        } else if args[i] == "-nf" {
            i += 1;
            config.colors[SCHEME_NORM][drw::COL_FG] = args[i].clone();
        } else if args[i] == "-sb" {
            i += 1;
            config.colors[SCHEME_SEL][drw::COL_BG] = args[i].clone();
        } else if args[i] == "-sf" {
            i += 1;
            config.colors[SCHEME_SEL][drw::COL_FG] = args[i].clone();
        } else if args[i] == "-w" {
            i += 1;
            embed = args[i].clone();
        } else {
            usage();
        }
        i += 1;
    }

    if unsafe { libc::setlocale(libc::LC_CTYPE, c"".as_ptr()).is_null() }
        || unsafe { XSupportsLocale() } == 0
    {
        eprintln!("warning: no locale support");
    }
    let Some(dpy) = NonNull::new(unsafe { XOpenDisplay(core::ptr::null_mut()) }) else {
        eprintln!("cannot open display");
        return ExitCode::FAILURE;
    };

    let screen = unsafe { default_screen(dpy.as_ptr()) };
    let root = unsafe { root_window(dpy.as_ptr(), screen) };
    let parentwin = if embed.is_empty() {
        root
    } else if let Ok(parentwin) = embed.parse::<Window>() {
        parentwin
    } else {
        root
    };

    let mut wa: XWindowAttributes = unsafe { core::mem::zeroed() };
    if unsafe { XGetWindowAttributes(dpy.as_ptr(), parentwin, &mut wa) } == 0 {
        eprintln!("could not get embedding window attributes: {:x}", parentwin);
        return ExitCode::FAILURE;
    }
    let (visual, depth, cmap, useargb) = xinitvisual(dpy, screen, root);
    let mut drw = drw::Drw::create(
        dpy,
        screen,
        root,
        wa.width as u32,
        wa.height as u32,
        visual,
        depth as u32,
        cmap,
    );
    if !drw.fontset_create(&config.fonts) {
        eprintln!("no fonts could be loaded");
        return ExitCode::FAILURE;
    }
    let lrpad = drw.fonts[0].h;

    let mut globals = Globals::new(dpy, drw);
    globals.embed = embed;
    globals.lrpad = lrpad as i32;
    globals.mon = mon;
    globals.screen = screen;
    globals.root = root;
    globals.parentwin = parentwin;
    globals.depth = depth;
    globals.cmap = cmap;
    globals.visual = visual;
    globals.useargb = useargb;
    globals.passwd = passwd;

    if fast && !stdin().is_terminal() {
        grabkeyboard(dpy, &globals);
        readstdin(&mut config, &mut globals);
    } else {
        readstdin(&mut config, &mut globals);
        grabkeyboard(dpy, &globals);
    };

    setup(&mut config, &mut globals);
    let code = run(&mut globals, &config);
    cleanup(globals);
    ExitCode::from(code as u8)
}
