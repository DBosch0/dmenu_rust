use std::{
    ffi::{CStr, CString},
    num::Wrapping,
    process::exit,
    ptr::NonNull,
    rc::Rc,
    sync::atomic::{AtomicPtr, AtomicU32, Ordering},
};

use crate::{config, external::*};

#[derive(Debug)]
pub(crate) struct Fnt {
    dpy: NonNull<Display>,
    pub(crate) h: u32,
    xfont: NonNull<XftFont>,
    pattern: Option<NonNull<FcPattern>>,
    // next: Box<Fnt>,
}

pub(crate) const COL_FG: usize = 0;
pub(crate) const COL_BG: usize = 1;
pub(crate) const COLORS_PER_SCHEME: usize = config::COLORS[0].len();

pub(crate) type Clr = XftColor;

/* Cache of codepoints that no fallback font could be found for, keyed by a
 * cheap hash so drw_text doesn't repeatedly pay for an XftFontMatch lookup
 * that is known to fail. Lives for the process lifetime; freed when the
 * owning Drw is dropped. */
static NOMATCHES: AtomicPtr<[u32; 128]> = AtomicPtr::new(core::ptr::null_mut());

#[derive(Debug)]
pub(crate) struct Drw {
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) dpy: NonNull<Display>,
    pub(crate) screen: i32,
    pub(crate) root: Window,
    pub(crate) drawable: Drawable,
    pub(crate) gc: GC,
    pub(crate) scheme: Rc<[Clr; COLORS_PER_SCHEME]>,
    pub(crate) fonts: Vec<Fnt>,
    pub(crate) visual: NonNull<Visual>,
    pub(crate) depth: u32,
    pub(crate) cmap: Colormap,
}

impl Drw {
    //TODO: maybe remove box
    pub(crate) fn create(
        dpy: NonNull<Display>,
        screen: i32,
        root: Window,
        w: u32,
        h: u32,
        visual: NonNull<Visual>,
        depth: u32,
        cmap: Colormap,
    ) -> Self {
        const LINE_SOLID: i32 = 0;
        const CAP_BUTT: i32 = 1;
        const JOIN_MITER: i32 = 0;

        let drawable = unsafe { XCreatePixmap(dpy.as_ptr(), root, w, h, depth) };

        let drw = Drw {
            w,
            h,
            dpy,
            screen,
            root,
            drawable,
            gc: unsafe { XCreateGC(dpy.as_ptr(), drawable, 0, core::ptr::null_mut()) },
            scheme: Rc::new([Clr::default(); COLORS_PER_SCHEME]),
            fonts: Vec::new(),
            visual,
            depth,
            cmap,
        };
        unsafe { XSetLineAttributes(dpy.as_ptr(), drw.gc, 1, LINE_SOLID, CAP_BUTT, JOIN_MITER) };

        drw
    }

    pub(crate) fn resize(&mut self, w: u32, h: u32) {
        self.w = w;
        self.h = h;
        if self.drawable != 0 {
            unsafe { XFreePixmap(self.dpy.as_ptr(), self.drawable) };
        }
        self.drawable = unsafe {
            XCreatePixmap(
                self.dpy.as_ptr(),
                self.root,
                w,
                h,
                self.depth, // default_depth(self.dpy.as_ptr(), self.screen) as u32,
            )
        };
    }

    pub(crate) fn fontset_create(&mut self, fonts: &[String]) -> bool {
        for font in fonts {
            if let Some(fnt) = Fnt::xfont_create(self, font, None) {
                self.fonts.push(fnt);
            }
        }
        !self.fonts.is_empty()
    }

    fn clr_create(&self, dest: &mut Clr, clrname: &str, alpha: u32) {
        if clrname.is_empty() {
            return;
        }
        let clrname_cstr = CString::new(clrname).expect("valid C-String");
        if unsafe {
            XftColorAllocName(
                self.dpy.as_ptr(),
                self.visual.as_ptr(), // default_visual(self.dpy.as_ptr(), self.screen),
                self.cmap,            // default_colormap(self.dpy.as_ptr(), self.screen),
                clrname_cstr.as_ptr(),
                dest,
            )
        } == 0
        {
            eprintln!("error, cannot allocate color '{}'", clrname);
            exit(1)
        }
        dest.pixel = (dest.pixel & 0x00ffffff) | (alpha as u64) << 24;
    }

    pub(crate) fn scm_create<const N: usize>(
        &mut self,
        clr_names: &[String; N],
        alphas: &[u32; N],
    ) -> Rc<[Clr; N]> {
        if clr_names.is_empty() || clr_names.len() < 2 {
            eprintln!(
                "Color Scheme incorrectly defined, needs at least 2 colors, got {} colors",
                clr_names.len()
            );
            exit(1)
        }
        let mut colors = [Clr::default(); N];
        for (i, color_name) in clr_names.iter().enumerate() {
            self.clr_create(&mut colors[i], color_name, alphas[i]);
        }

        Rc::new(colors)
    }

    pub(crate) fn set_scheme(&mut self, scm: Rc<[Clr; COLORS_PER_SCHEME]>) {
        self.scheme = scm;
    }

    pub(crate) fn rect(&self, x: i32, y: i32, w: u32, h: u32, filled: bool, invert: bool) {
        if self.scheme.is_empty() {
            return;
        }
        unsafe {
            XSetForeground(
                self.dpy.as_ptr(),
                self.gc,
                if invert {
                    self.scheme[COL_BG].pixel
                } else {
                    self.scheme[COL_FG].pixel
                },
            )
        };
        if filled {
            unsafe { XFillRectangle(self.dpy.as_ptr(), self.drawable, self.gc, x, y, w, h) };
        } else {
            unsafe {
                XDrawRectangle(
                    self.dpy.as_ptr(),
                    self.drawable,
                    self.gc,
                    x,
                    y,
                    w - 1,
                    h - 1,
                )
            };
        }
    }

    pub(crate) fn text(
        &mut self,
        mut x: i32,
        y: i32,
        mut w: u32,
        h: u32,
        lpad: u32,
        mut text: &str,
        invert: u32,
    ) -> i32 {
        let mut ty;
        let mut ellipsis_x = 0;
        let mut tmpw: u32 = 0;
        let mut ew: u32;
        let mut ellipsis_w = 0u32;
        let mut ellipsis_len: u32;
        let mut hash: Wrapping<u32>;
        let mut h0: u32;
        let mut h1: u32;
        let mut d: *mut XftDraw = core::ptr::null_mut();
        let mut used_font: usize;
        let mut next_font: Option<usize>;
        let mut utf8_str_len: i32;
        let render = x != 0 || y != 0 || w != 0 || h != 0;
        let mut code_point: u32 = 0;
        let mut utf8_str: &str;
        let mut fccharset: *mut FcCharSet;
        let mut fcpattern: *mut FcPattern;
        let mut match_: *mut FcPattern;
        let mut result: i32 = 0;
        let mut char_exists = false;
        let mut overflow = false;

        // constants and statics
        static ELLIPSIS_WIDTH: AtomicU32 = AtomicU32::new(0);
        let mut nomatches = NOMATCHES.load(Ordering::Relaxed);
        if nomatches.is_null() {
            nomatches = Box::into_raw(Box::new([0; 128]));
            NOMATCHES.store(nomatches, Ordering::Relaxed);
        }

        if (render && (self.scheme.is_empty() || w == 0))
            || text.is_empty()
            || self.fonts.is_empty()
        {
            return 0;
        }

        if !render {
            w = if invert != 0 { invert } else { !invert };
        } else {
            unsafe {
                XSetForeground(
                    self.dpy.as_ptr(),
                    self.gc,
                    self.scheme[if invert != 0 { COL_FG } else { COL_BG }].pixel,
                )
            };
            unsafe { XFillRectangle(self.dpy.as_ptr(), self.drawable, self.gc, x, y, w, h) };
            d = unsafe {
                XftDrawCreate(
                    self.dpy.as_ptr(),
                    self.drawable,
                    self.visual.as_ptr(),
                    self.cmap,
                )
            };
            if w < lpad {
                return x + w as i32;
            }
            x += lpad as i32;
            w -= lpad;
        }

        used_font = 0;
        if ELLIPSIS_WIDTH.load(Ordering::Relaxed) == 0 && render {
            ELLIPSIS_WIDTH.store(self.fontset_get_width("..."), Ordering::Relaxed);
        }

        loop {
            ew = 0;
            ellipsis_len = 0;
            utf8_str_len = 0;
            utf8_str = text;
            next_font = None;
            while !text.is_empty() {
                // `text` is always valid UTF-8 (it's a &str), so unlike the C
                // original there is no decode-error/replacement-char path here.
                let ch = text.chars().next().unwrap();
                let char_len = ch.len_utf8();
                code_point = ch as u32;
                for cur_font in 0..self.fonts.len() {
                    char_exists = char_exists
                        || unsafe {
                            XftCharExists(
                                self.dpy.as_ptr(),
                                self.fonts[cur_font].xfont.as_ptr(),
                                code_point,
                            )
                        } != 0;
                    if char_exists {
                        self.fonts[cur_font].get_exts(text, char_len as u32, Some(&mut tmpw), None);
                        if ew + ELLIPSIS_WIDTH.load(Ordering::Relaxed) <= w {
                            // keep track where the ellipsis still fits
                            ellipsis_x = x + ew as i32;
                            ellipsis_w = w - ew;
                            ellipsis_len = utf8_str_len as u32;
                        }

                        if ew + tmpw > w {
                            overflow = true;
                            // called from drw_fontset_getwidth_clamp():
                            // it wants the width AFTER the overflow
                            if !render {
                                x += tmpw as i32;
                            } else {
                                utf8_str_len = ellipsis_len as i32;
                            }
                        } else if cur_font == used_font {
                            text = &text[char_len..];
                            utf8_str_len += char_len as i32;
                            ew += tmpw;
                        } else {
                            next_font = Some(cur_font);
                        }
                        break;
                    }
                }

                if overflow || !char_exists || next_font.is_some() {
                    break;
                } else {
                    char_exists = false;
                }
            }

            if utf8_str_len != 0 {
                if render {
                    ty = y
                        + (h - self.fonts[used_font].h) as i32 / 2
                        + unsafe { self.fonts[used_font].xfont.as_ref().ascent };
                    unsafe {
                        XftDrawStringUtf8(
                            d,
                            &self.scheme[if invert != 0 { COL_BG } else { COL_FG }],
                            self.fonts[used_font].xfont.as_ptr(),
                            x,
                            ty,
                            utf8_str.as_ptr(),
                            utf8_str_len,
                        )
                    };
                }
                x += ew as i32;
                w -= ew;
            }
            if render && overflow {
                self.text(ellipsis_x, y, ellipsis_w, h, 0, "...", invert);
            }

            if text.is_empty() || overflow {
                break;
            } else if let Some(nf) = next_font {
                char_exists = false;
                used_font = nf;
            } else {
                // Regardless of whether or not a fallback font is found, the
                // character must be drawn.
                char_exists = true;

                hash = Wrapping(code_point);
                hash = ((hash >> 16) ^ hash) * Wrapping(0x21F0AAAD);
                hash = ((hash >> 15) ^ hash) * Wrapping(0xD35A2D97);
                h0 = ((hash.0 >> 15) ^ hash.0) % unsafe { &*nomatches }.len() as u32;
                h1 = (hash.0 >> 17) % unsafe { &*nomatches }.len() as u32;
                // avoid expensive XftFontMatch call when we know we won't find a match
                if unsafe { &*nomatches }[h0 as usize] == code_point
                    || unsafe { &*nomatches }[h1 as usize] == code_point
                {
                    used_font = 0;
                    continue;
                }

                fccharset = unsafe { FcCharSetCreate() };
                unsafe { FcCharSetAddChar(fccharset, code_point) };

                if self.fonts[0].pattern.is_none() {
                    // Refer to the comment in xfont_create for more information.
                    eprintln!("the first font in the cache must be loaded from a font string");
                    exit(1);
                }

                const FC_CHARSET: &CStr = c"charset";
                const FC_SCALABLE: &CStr = c"scalable";
                const FC_TRUE: i32 = 1;
                const FC_MATCH_PATTERN: i32 = 0;

                fcpattern = unsafe { FcPatternDuplicate(self.fonts[0].pattern.unwrap().as_ptr()) };
                unsafe { FcPatternAddCharSet(fcpattern, FC_CHARSET.as_ptr(), fccharset) };
                unsafe { FcPatternAddBool(fcpattern, FC_SCALABLE.as_ptr(), FC_TRUE) };

                unsafe { FcConfigSubstitute(core::ptr::null_mut(), fcpattern, FC_MATCH_PATTERN) };
                unsafe { FcDefaultSubstitute(fcpattern) };
                match_ =
                    unsafe { XftFontMatch(self.dpy.as_ptr(), self.screen, fcpattern, &mut result) };

                unsafe { FcCharSetDestroy(fccharset) };
                unsafe { FcPatternDestroy(fcpattern) };

                if !match_.is_null() {
                    let candidate = Fnt::xfont_create(self, "", NonNull::new(match_)).filter(
                        |f| unsafe {
                            XftCharExists(self.dpy.as_ptr(), f.xfont.as_ptr(), code_point)
                        } != 0,
                    );
                    if let Some(new_font) = candidate {
                        self.fonts.push(new_font);
                        used_font = self.fonts.len() - 1;
                    } else {
                        let nm = unsafe { &mut *nomatches };
                        nm[if nm[h0 as usize] != 0 { h1 } else { h0 } as usize] = code_point;
                        used_font = 0;
                    }
                }
            }
        }

        if !d.is_null() {
            unsafe { XftDrawDestroy(d) };
        }
        x + if render { w } else { 0 } as i32
    }

    pub(crate) fn fontset_get_width(&mut self, text: &str) -> u32 {
        if self.fonts.is_empty() || text.is_empty() {
            return 0;
        }
        self.text(0, 0, 0, 0, 0, text, 0) as u32
    }

    pub(crate) fn fontset_get_width_clamp(&mut self, text: &str, n: u32) -> u32 {
        let tmp = if !self.fonts.is_empty() && !text.is_empty() && n != 0 {
            self.text(0, 0, 0, 0, 0, text, n) as u32
        } else {
            0
        };
        n.min(tmp)
    }

    pub(crate) fn map(&self, win: Window, x: i32, y: i32, w: u32, h: u32) {
        unsafe {
            XCopyArea(
                self.dpy.as_ptr(),
                self.drawable,
                win,
                self.gc,
                x,
                y,
                w,
                h,
                x,
                y,
            )
        };
        unsafe { XSync(self.dpy.as_ptr(), 0) };
    }
}

impl Fnt {
    fn xfont_create(
        drw: &Drw,
        fontname: &str,
        fontpattern: Option<NonNull<FcPattern>>,
    ) -> Option<Self> {
        let xfont: *mut XftFont;
        let mut pattern: *mut FcPattern = core::ptr::null_mut();

        if !fontname.is_empty() {
            // Using the pattern found at font->xfont->pattern does not yield the
            // same substitution results as using the pattern returned by
            // FcNameParse; using the latter results in the desired fallback
            // behaviour whereas the former just results in missing-character
            // rectangles being drawn, at least with some fonts.
            let fontname_cstr = CString::new(fontname).expect("valid C String");
            xfont =
                unsafe { XftFontOpenName(drw.dpy.as_ptr(), drw.screen, fontname_cstr.as_ptr()) };
            if xfont.is_null() {
                eprintln!("error, cannot load font from name: '{}'", fontname);
                return None;
            }
            pattern = unsafe { FcNameParse(fontname_cstr.as_ptr().cast()) };
            if pattern.is_null() {
                eprintln!("error, cannot parse font name to pattern : '{}'", fontname);
                unsafe { XftFontClose(drw.dpy.as_ptr(), xfont) };
                return None;
            }
        } else if let Some(fontpattern) = fontpattern {
            xfont = unsafe { XftFontOpenPattern(drw.dpy.as_ptr(), fontpattern.as_ptr()) };
            if xfont.is_null() {
                eprintln!("error, cannot load font from pattern");
                return None;
            }
        } else {
            eprintln!("no font specified");
            exit(1);
        }

        Some(Fnt {
            dpy: drw.dpy,
            h: (unsafe { &*xfont }.ascent + unsafe { &*xfont }.descent) as u32,
            xfont: unsafe { NonNull::new_unchecked(xfont) },
            pattern: NonNull::new(pattern),
        })
    }

    pub(crate) fn get_exts(
        &mut self,
        text: &str,
        len: u32,
        w: Option<&mut u32>,
        h: Option<&mut u32>,
    ) {
        let mut ext: XGlyphInfo = unsafe { core::mem::zeroed() };
        if text.is_empty() {
            return;
        }

        unsafe {
            XftTextExtentsUtf8(
                self.dpy.as_ptr(),
                self.xfont.as_ptr(),
                text.as_ptr(),
                len as i32,
                &mut ext,
            )
        };
        if let Some(w) = w {
            *w = ext.x_off as u32;
        }
        if let Some(h) = h {
            *h = self.h;
        }
    }
}

impl Drop for Fnt {
    fn drop(&mut self) {
        if let Some(pattern) = self.pattern {
            unsafe { FcPatternDestroy(pattern.as_ptr()) }
        }
        unsafe { XftFontClose(self.dpy.as_ptr(), self.xfont.as_ptr()) };
    }
}

impl Drop for Drw {
    fn drop(&mut self) {
        unsafe { XFreePixmap(self.dpy.as_ptr(), self.drawable) };
        unsafe { XFreeGC(self.dpy.as_ptr(), self.gc) };
        let nomatches = NOMATCHES.swap(core::ptr::null_mut(), Ordering::Relaxed);
        if !nomatches.is_null() {
            drop(unsafe { Box::from_raw(nomatches) });
        }
    }
}
