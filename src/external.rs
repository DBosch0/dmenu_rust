use std::ffi::{c_char, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_ushort, c_void};

// FFI Type definitions
pub(crate) type XID = c_ulong;
pub(crate) type Cursor = XID;
pub(crate) type Window = XID;
pub(crate) type Drawable = XID;
pub(crate) type Pixmap = XID;
pub(crate) type Colormap = XID;
pub(crate) type Font = XID;
pub(crate) type KeySym = XID;
pub(crate) type PictFormat = XID;
pub(crate) type VisualID = c_ulong;
pub(crate) type Atom = c_ulong;
pub(crate) type Time = c_ulong;
pub(crate) type Status = c_int;
pub(crate) type Display = _XDisplay;
pub(crate) type GC = *mut _XGC;
pub(crate) type XPointer = *mut c_char;
pub(crate) type XIC = *mut _XIC;
pub(crate) type XIM = *mut _XIM;
pub(crate) type XrmDatabase = *mut _XrmHashBucketRec;

// X11 Structures

pub(crate) enum _XGC {}
pub(crate) enum _XIC {}
pub(crate) enum _XIM {}
pub(crate) enum _XPrivate {}
pub(crate) enum _XrmHashBucketRec {}
pub(crate) enum XftDraw {}
pub(crate) enum _XDisplay {}

#[repr(C)]
struct _XPrivDisplay {
    ext_data: *mut XExtData,
    private1: *mut _XPrivate,
    fd: c_int,
    private2: c_int,
    proto_major_version: c_int, /* major version of server's X protocol */
    proto_minor_version: c_int, /* minor version of servers X protocol */
    vendor: *mut c_char,        /* vendor of the server hardware */
    private3: XID,
    private4: XID,
    private5: XID,
    private6: c_int,
    resource_alloc: fn(display: *mut _XDisplay) -> XID, /* allocator function */
    byte_order: c_int,                                  /* screen byte order, LSBFirst, MSBFirst */
    bitmap_unit: c_int,                                 /* padding and data requirements */
    bitmap_pad: c_int,                                  /* padding requirements on bitmaps */
    bitmap_bit_order: c_int,                            /* LeastSignificant or MostSignificant */
    nformats: c_int,                                    /* LeastSignificant or MostSignificant */
    pixmap_format: *mut ScreenFormat,                   /* pixmap format list */
    private8: c_int,
    release: c_int, /* release of the server */
    private9: *mut _XPrivate,
    private10: *mut _XPrivate,
    qlen: c_int,                /* Length of input event queue */
    last_request_read: c_ulong, /* seq number of last event read */
    request: c_ulong,           /* sequence number of last request. */
    private11: XPointer,
    private12: XPointer,
    private13: XPointer,
    private14: XPointer,
    max_request_size: c_uint, /* maximum number 32 bit words in request*/
    db: *mut _XrmHashBucketRec,
    private15: fn(display: *mut _XDisplay) -> c_int,
    display_name: *mut c_char, /* "host:display" string used on this connect*/
    default_screen: c_int,     /* default screen for operations */
    nscreens: c_int,           /* number of screens on this server*/
    screens: *mut Screen,      /* pointer to list of screens */
    motion_buffer: c_ulong,    /* size of motion buffer */
    private16: c_ulong,
    min_keycode: c_int, /* minimum defined keycode */
    max_keycode: c_int, /* maximum defined keycode */
    private17: XPointer,
    private18: XPointer,
    private19: c_int,
    xdefaults: *mut c_char, /* contents of defaults from server */
}

#[repr(C)]
struct ScreenFormat {
    ext_data: *mut XExtData,
    depth: c_int,
    bits_per_pixel: c_int,
    scanline_pad: c_int,
}

#[repr(C)]
struct XExtData {
    number: c_int,
    next: *mut XExtData,
    free_private: extern "C" fn(extention: *mut XExtData) -> c_int,
    private_data: XPointer,
}

#[repr(C)]
struct Screen {
    ext_data: *mut XExtData,
    display: *mut Display,
    root: Window,
    width: c_int,
    height: c_int,
    mwidth: c_int,
    mheight: c_int,
    ndepth: c_int,
    depths: *mut Depth,
    root_depth: c_int,
    root_visual: *mut Visual,
    default_gc: GC,
    cmap: Colormap,
    white_pixel: c_ulong,
    black_pixel: c_ulong,
    max_maps: c_int,
    min_maps: c_int,
    backing_store: c_int,
    save_unders: c_int, // Bool
    root_input_mask: c_int,
}

#[repr(C)]
struct Depth {
    depth: c_int,
    nvisuals: c_int,
    visuals: *mut Visual,
}

#[repr(C)]
pub(crate) struct Visual {
    ext_data: *mut XExtData,
    visualid: VisualID,
    class: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    bits_per_rgb: c_int,
    map_entries: c_int,
}

#[repr(C)]
pub(crate) struct XGCValues {
    function: c_int,
    plane_mask: c_ulong,
    foreground: c_ulong,
    background: c_ulong,
    line_width: c_int,
    line_style: c_int,
    cap_style: c_int,
    join_style: c_int,
    fill_style: c_int,
    fill_rule: c_int,
    arc_mode: c_int,
    tile: Pixmap,
    stipple: Pixmap,
    ts_x_origin: c_int,
    ts_y_origin: c_int,
    font: Font,
    subwindow_mode: c_int,
    graphics_exposure: c_int, //Bool
    clip_x_origin: c_int,
    clip_y_origin: c_int,
    clip_mask: Pixmap,
    dash_offset: c_int,
    dashes: c_char,
}

#[repr(C)]
pub(crate) struct XGlyphInfo {
    width: c_ushort,
    height: c_ushort,
    x: c_short,
    y: c_short,
    pub(crate) x_off: c_short,
    y_off: c_short,
}

#[repr(C)]
pub(crate) struct XWindowAttributes {
    pub(crate) x: c_int,
    pub(crate) y: c_int,
    pub(crate) width: c_int,
    pub(crate) height: c_int,
    border_width: c_int,
    depth: c_int,
    visual: *mut Visual,
    root: Window,
    class: c_int,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_uint,
    backing_pixel: c_uint,
    save_under: c_int,
    colormap: Colormap,
    map_installed: c_int,
    map_state: c_int,
    all_event_mask: c_long,
    your_event_mask: c_long,
    do_not_propogate_mask: c_long,
    override_redirect: c_int,
    screen: *mut Screen,
}

#[repr(C)]
pub(crate) struct XClassHint {
    pub(crate) res_name: *mut c_char,
    pub(crate) res_class: *mut c_char,
}

#[repr(C)]
pub(crate) struct XSetWindowAttributes {
    background_pixmap: Pixmap,
    pub(crate) background_pixel: c_ulong,
    border_pixmap: Pixmap,
    pub(crate) border_pixel: c_ulong,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: c_int,
    pub(crate) event_mask: c_long,
    do_not_propagate_mask: c_long,
    pub(crate) override_redirect: c_int,
    pub(crate) colormap: Colormap,
    cursor: Cursor,
}

#[repr(C)]
pub(crate) union XEvent {
    pub(crate) typ: i32,
    pub(crate) xdestroywindow: XDestroyWindowEvent,
    pub(crate) xexpose: XExposeEvent,
    pub(crate) xfocus: XFocusChangeEvent,
    pub(crate) xkey: XKeyEvent,
    pub(crate) xselection: XSelectionEvent,
    pub(crate) xvisibility: XVisibilityEvent,
    pad: [i64; 24],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XDestroyWindowEvent {
    typ: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    event: Window,
    pub(crate) window: Window,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XExposeEvent {
    typ: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: Window,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    pub(crate) count: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XFocusChangeEvent {
    typ: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    pub(crate) window: Window,
    mode: c_int,
    detail: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XKeyEvent {
    typ: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: Window,
    root: Window,
    subwindow: Window,
    time: Time,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    pub(crate) state: c_uint,
    keycode: c_uint,
    same_screen: c_int,
}
pub(crate) type XKeyPressedEvent = XKeyEvent;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XSelectionEvent {
    typ: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    requestor: Window,
    selection: Atom,
    target: Atom,
    pub(crate) property: Atom,
    time: Time,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct XVisibilityEvent {
    typ: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    requestor: Window,
    pub(crate) state: c_int,
}

#[repr(C)]
pub(crate) struct XrmValue {
    size: c_uint,
    pub(crate) addr: XPointer,
}

#[repr(C)]
pub(crate) struct XVisualInfo {
    pub(crate) visual: *mut Visual,
    visualid: VisualID,
    pub(crate) screen: c_int,
    pub(crate) depth: c_int,
    pub(crate) class: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    colormap_size: c_int,
    bits_per_rgb: c_int,
}

// X11 External Functions
#[link(name = "X11")]
unsafe extern "C" {
    pub(crate) fn XCreatePixmap(
        display: *mut Display,
        d: Drawable,
        width: u32,
        height: u32,
        depth: u32,
    ) -> Pixmap;
    pub(crate) fn XCreateGC(
        display: *mut Display,
        d: Drawable,
        valuemask: c_ulong,
        values: *mut XGCValues,
    ) -> GC;
    pub(crate) fn XSetLineAttributes(
        display: *mut Display,
        gc: GC,
        line_width: c_uint,
        line_style: c_int,
        cap_style: c_int,
        join_style: c_int,
    ) -> c_int;
    pub(crate) fn XFreePixmap(display: *mut Display, pixmap: Pixmap) -> c_int;
    pub(crate) fn XFreeGC(display: *mut Display, gc: GC) -> c_int;
    pub(crate) fn XSetForeground(display: *mut Display, gc: GC, foreground: c_ulong) -> c_int;
    pub(crate) fn XFillRectangle(
        display: *mut Display,
        d: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub(crate) fn XDrawRectangle(
        display: *mut Display,
        d: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub(crate) fn XCopyArea(
        display: *mut Display,
        src: Drawable,
        dest: Drawable,
        gc: GC,
        src_x: c_int,
        src_y: c_int,
        width: c_uint,
        height: c_uint,
        dest_x: c_int,
        dest_y: c_int,
    ) -> c_int;
    pub(crate) fn XSync(display: *mut Display, discard: c_int) -> c_int;
    pub(crate) fn XSupportsLocale() -> c_int;
    pub(crate) fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    pub(crate) fn XGetWindowAttributes(
        display: *mut Display,
        w: Window,
        window_attributes_return: *mut XWindowAttributes,
    ) -> c_int;
    pub(crate) fn XGrabKeyboard(
        display: *mut Display,
        grab_window: Window,
        owner_event: c_int,
        pointer_mode: c_int,
        keyboard_mode: c_int,
        time: Time,
    ) -> c_int;
    pub(crate) fn XInternAtom(
        display: *mut Display,
        atom_name: *const c_char,
        only_if_exists: c_int,
    ) -> Atom;
    pub(crate) fn XCreateWindow(
        display: *mut Display,
        parent: Window,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        border_width: u32,
        depth: i32,
        class: u32,
        visual: *mut Visual,
        value_mask: u64,
        attributes: *mut XSetWindowAttributes,
    ) -> Window;
    pub(crate) fn XSetClassHint(
        display: *mut Display,
        w: Window,
        class_hints: *mut XClassHint,
    ) -> c_int;
    pub(crate) fn XOpenIM(
        display: *mut Display,
        rdb: *mut _XrmHashBucketRec,
        res_name: *mut c_char,
        res_class: *mut c_char,
    ) -> XIM;
    pub(crate) fn XCreateIC(im: XIM, ...) -> XIC;
    pub(crate) fn XMapRaised(display: *mut Display, w: Window) -> c_int;
    pub(crate) fn XReparentWindow(
        display: *mut Display,
        w: Window,
        parent: Window,
        x: c_int,
        y: c_int,
    ) -> c_int;
    pub(crate) fn XSelectInput(display: *mut Display, w: Window, event_mask: c_long) -> c_int;
    pub(crate) fn XQueryTree(
        display: *mut Display,
        w: Window,
        root_return: *mut Window,
        parent_return: *mut Window,
        children_return: *mut *mut Window,
        nchildren_return: *mut c_uint,
    ) -> c_int;
    pub(crate) fn XFree(data: *mut c_void) -> c_int;
    pub(crate) fn XGetInputFocus(
        display: *mut Display,
        focus_return: *mut Window,
        rever_to_return: *mut c_int,
    ) -> c_int;
    pub(crate) fn XSetInputFocus(
        display: *mut Display,
        focus: Window,
        revert_to: c_int,
        time: Time,
    ) -> c_int;
    pub(crate) fn XNextEvent(Display: *mut Display, event_return: *mut XEvent) -> c_int;
    pub(crate) fn XFilterEvent(event: *mut XEvent, window: Window) -> c_int;
    pub(crate) fn XRaiseWindow(display: *mut Display, w: Window) -> c_int;
    pub(crate) fn XUngrabKeyboard(display: *mut Display, time: Time) -> c_int;
    pub(crate) fn XCloseDisplay(display: *mut Display) -> c_int;
    pub(crate) fn XGetWindowProperty(
        display: *mut Display,
        w: Window,
        property: Atom,
        long_offset: c_long,
        long_length: c_long,
        delete: c_int,
        req_type: Atom,
        actual_return_type: *mut Atom,
        actual_return_format: *mut c_int,
        nitems_return: *mut c_ulong,
        bytes_after_return: &mut c_ulong,
        prop_return: *mut *mut c_char,
    ) -> c_int;
    pub(crate) fn XmbLookupString(
        ic: XIC,
        event: *mut XKeyPressedEvent,
        buffer_return: *mut i8,
        bytes_buffer: c_int,
        keysym_return: *mut KeySym,
        status_return: *mut Status,
    ) -> c_int;
    pub(crate) fn XConvertSelection(
        display: *mut Display,
        selection: Atom,
        target: Atom,
        property: Atom,
        requestor: Window,
        time: Time,
    ) -> c_int;
    pub(crate) fn XQueryPointer(
        display: *mut Display,
        w: Window,
        root_return: *mut Window,
        child_return: *mut Window,
        root_x_return: *mut c_int,
        root_y_return: *mut c_int,
        win_x_return: *mut c_int,
        win_y_return: *mut c_int,
        mask_return: *mut c_uint,
    ) -> c_int;
    pub(crate) fn XrmInitialize();
    pub(crate) fn XrmGetStringDatabase(data: *const c_char) -> XrmDatabase;
    pub(crate) fn XrmGetResource(
        database: XrmDatabase,
        str_name: *const c_char,
        str_class: *const c_char,
        str_type_return: *mut *mut c_char,
        value_return: *mut XrmValue,
    ) -> c_int;
    pub(crate) fn XrmDestroyDatabase(database: XrmDatabase);
    pub(crate) fn XResourceManagerString(display: *mut Display) -> *mut c_char;
    pub(crate) fn XGetVisualInfo(
        display: *mut Display,
        vinfo_mask: c_long,
        vinfo_template: *mut XVisualInfo,
        nitems_retun: *mut c_int,
    ) -> *mut XVisualInfo;
    pub(crate) fn XCreateColormap(
        display: *mut Display,
        w: Window,
        visual: *mut Visual,
        alloc: c_int,
    ) -> Colormap;
}

// X11 Macros expressed as helper functions
#[inline(always)]
unsafe fn screen_of_display(dpy: *mut Display, src: i32) -> *mut Screen {
    assert!(src >= 0, "src cannot be negative");
    let priv_dpy: *mut _XPrivDisplay = dpy.cast();
    unsafe { (*priv_dpy).screens.add(src as usize) }
}

#[inline(always)]
pub(crate) unsafe fn default_depth(dpy: *mut Display, src: i32) -> i32 {
    (unsafe { &*screen_of_display(dpy, src) }).root_depth
}

#[inline(always)]
pub(crate) unsafe fn default_visual(dpy: *mut Display, src: i32) -> *mut Visual {
    (unsafe { &*screen_of_display(dpy, src) }).root_visual
}

#[inline(always)]
pub(crate) unsafe fn default_colormap(dpy: *mut Display, src: i32) -> Colormap {
    (unsafe { &*screen_of_display(dpy, src) }).cmap
}

#[inline(always)]
pub(crate) unsafe fn root_window(dpy: *mut Display, src: i32) -> Window {
    (unsafe { &*screen_of_display(dpy, src) }).root
}

#[inline(always)]
pub(crate) unsafe fn default_screen(dpy: *mut Display) -> i32 {
    let priv_dpy: *mut _XPrivDisplay = dpy.cast();
    { unsafe { &*priv_dpy } }.default_screen
}

#[inline(always)]
pub(crate) unsafe fn default_root_window(dpy: *mut Display) -> Window {
    (unsafe { &*screen_of_display(dpy, default_screen(dpy)) }).root
}

// Xft Structures
#[repr(C)]
pub(crate) struct XftFont {
    pub(crate) ascent: c_int,
    pub(crate) descent: c_int,
    height: c_int,
    max_advance_width: c_int,
    charset: *mut FcCharSet,
    pattern: *mut FcPattern,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct XftColor {
    pub(crate) pixel: c_ulong,
    color: XRenderColor,
}

// From Xrender extension not Xft, but we do not link with Xrender, we placed here for organization.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct XRenderColor {
    red: c_ushort,
    green: c_ushort,
    blue: c_ushort,
    alpha: c_ushort,
}

// Xft external functions
#[link(name = "Xft")]
unsafe extern "C" {
    pub(crate) fn XftFontOpenName(
        dpy: *mut Display,
        screen: c_int,
        name: *const c_char,
    ) -> *mut XftFont;
    pub(crate) fn XftFontOpenPattern(dpy: *mut Display, pattern: *mut FcPattern) -> *mut XftFont;
    pub(crate) fn XftFontClose(dpy: *mut Display, fnt: *mut XftFont);
    pub(crate) fn XftColorAllocName(
        dpy: *mut Display,
        visual: *const Visual,
        cmap: Colormap,
        name: *const c_char,
        result: *mut XftColor,
    ) -> c_int;
    pub(crate) fn XftDrawCreate(
        dpy: *mut Display,
        drawable: Drawable,
        visual: *mut Visual,
        colormap: Colormap,
    ) -> *mut XftDraw;
    pub(crate) fn XftCharExists(dpy: *mut Display, fnt: *mut XftFont, ucs4: c_uint) -> c_int;
    pub(crate) fn XftDrawStringUtf8(
        draw: *mut XftDraw,
        color: *const XftColor,
        fnt: *mut XftFont,
        x: c_int,
        y: c_int,
        string: *const c_uchar,
        len: c_int,
    );
    pub(crate) fn XftFontMatch(
        dpy: *mut Display,
        screen: c_int,
        pattern: *const FcPattern,
        result: *mut c_int,
    ) -> *mut FcPattern;
    pub(crate) fn XftDrawDestroy(draw: *mut XftDraw);
    pub(crate) fn XftTextExtentsUtf8(
        dpy: *mut Display,
        fnt: *mut XftFont,
        string: *const c_uchar,
        len: c_int,
        extents: *mut XGlyphInfo,
    );
}

// Font Config structures
pub(crate) enum FcCharSet {}
pub(crate) enum FcPattern {}

// Font Config external functions
#[link(name = "fontconfig")]
unsafe extern "C" {
    pub(crate) fn FcNameParse(name: *const c_uchar) -> *mut FcPattern;
    pub(crate) fn FcPatternDestroy(p: *mut FcPattern);
    pub(crate) fn FcCharSetCreate() -> *mut FcCharSet;
    pub(crate) fn FcCharSetAddChar(fcs: *mut FcCharSet, ucs4: c_uint) -> c_int;
    pub(crate) fn FcPatternDuplicate(p: *const FcPattern) -> *mut FcPattern;
    pub(crate) fn FcPatternAddCharSet(
        p: *mut FcPattern,
        object: *const c_char,
        c: *const FcCharSet,
    ) -> c_int;
    pub(crate) fn FcPatternAddBool(p: *mut FcPattern, object: *const c_char, b: c_int) -> c_int;
    pub(crate) fn FcConfigSubstitute(config: *mut (), p: *mut FcPattern, kind: c_int) -> c_int;
    pub(crate) fn FcDefaultSubstitute(pattern: *mut FcPattern);
    pub(crate) fn FcCharSetDestroy(fcs: *mut FcCharSet);

}

// Xinerama

#[cfg(feature = "xinerama")]
#[repr(C)]
pub(crate) struct XineramaScreenInfo {
    screen_number: c_int,
    pub(crate) x_org: c_short,
    pub(crate) y_org: c_short,
    pub(crate) width: c_short,
    pub(crate) height: c_short,
}

#[cfg(feature = "xinerama")]
#[link(name = "Xinerama")]
unsafe extern "C" {
    pub(crate) fn XineramaQueryScreens(
        dpy: *mut Display,
        number: *mut c_int,
    ) -> *mut XineramaScreenInfo;
}

// XRender
#[repr(C)]
pub(crate) struct XRenderPictFormat {
    id: PictFormat,
    pub(crate) typ: c_int,
    depth: c_int,
    pub(crate) direct: XRenderDirectFormat,
    colormap: Colormap,
}

#[repr(C)]
pub(crate) struct XRenderDirectFormat {
    red: c_short,
    red_mask: c_short,
    green: c_short,
    green_mask: c_short,
    blue: c_short,
    blue_mask: c_short,
    alpha: c_short,
    pub(crate) alpha_mask: c_short,
}

#[link(name = "Xrender")]
unsafe extern "C" {
    pub(crate) fn XRenderFindVisualFormat(
        dpy: *mut Display,
        visual: *const Visual,
    ) -> *mut XRenderPictFormat;
}

// Keycode Constants
#[allow(non_upper_case_globals)]
pub(crate) mod keycodes {
    pub(crate) const XK_a: u64 = 0x0061; // U+0061 LATIN SMALL LETTER A 
    pub(crate) const XK_b: u64 = 0x0062; // U+0062 LATIN SMALL LETTER B 
    pub(crate) const XK_c: u64 = 0x0063; // U+0063 LATIN SMALL LETTER C 
    pub(crate) const XK_d: u64 = 0x0064; // U+0064 LATIN SMALL LETTER D 
    pub(crate) const XK_e: u64 = 0x0065; // U+0065 LATIN SMALL LETTER E 
    pub(crate) const XK_f: u64 = 0x0066; // U+0066 LATIN SMALL LETTER F 
    pub(crate) const XK_g: u64 = 0x0067; // U+0067 LATIN SMALL LETTER G 
    pub(crate) const XK_h: u64 = 0x0068; // U+0068 LATIN SMALL LETTER H 
    pub(crate) const XK_i: u64 = 0x0069; // U+0069 LATIN SMALL LETTER I 
    pub(crate) const XK_j: u64 = 0x006a; // U+006A LATIN SMALL LETTER J 
    pub(crate) const XK_k: u64 = 0x006b; // U+006B LATIN SMALL LETTER K 
    pub(crate) const XK_l: u64 = 0x006c; // U+006C LATIN SMALL LETTER L 
    pub(crate) const XK_m: u64 = 0x006d; // U+006D LATIN SMALL LETTER M 
    pub(crate) const XK_n: u64 = 0x006e; // U+006E LATIN SMALL LETTER N 
    pub(crate) const XK_p: u64 = 0x0070; // U+0070 LATIN SMALL LETTER P 
    pub(crate) const XK_u: u64 = 0x0075; // U+0075 LATIN SMALL LETTER U 
    pub(crate) const XK_w: u64 = 0x0077; // U+0077 LATIN SMALL LETTER W 
    pub(crate) const XK_y: u64 = 0x0079; // U+0079 LATIN SMALL LETTER Y 
    pub(crate) const XK_G: u64 = 0x0047; // U+0047 LATIN CAPITAL LETTER G 
    pub(crate) const XK_J: u64 = 0x004a; // U+004A LATIN CAPITAL LETTER J 
    pub(crate) const XK_M: u64 = 0x004d; // U+004D LATIN CAPITAL LETTER M 
    pub(crate) const XK_Y: u64 = 0x0059; // U+0059 LATIN CAPITAL LETTER Y 
    pub(crate) const XK_Home: u64 = 0xff50;
    pub(crate) const XK_Left: u64 = 0xff51; // Move left, left arrow
    pub(crate) const XK_Up: u64 = 0xff52; // Move up, up arrow 
    pub(crate) const XK_Right: u64 = 0xff53; // Move right, right arrow
    pub(crate) const XK_Down: u64 = 0xff54; // Move down, down arrow 
    pub(crate) const XK_Prior: u64 = 0xff55; // Prior, previous 
    pub(crate) const XK_Next: u64 = 0xff56; // Next 
    pub(crate) const XK_End: u64 = 0xff57; // EOL
    pub(crate) const XK_BackSpace: u64 = 0xff08; // U+0008 BACKSPACE 
    pub(crate) const XK_Tab: u64 = 0xff09; // U+0009 CHARACTER TABULATION 
    pub(crate) const XK_Return: u64 = 0xff0d; // U+000D CARRIAGE RETURN 
    pub(crate) const XK_Escape: u64 = 0xff1b; // U+001B ESCAPE 
    pub(crate) const XK_Delete: u64 = 0xffff; // U+007F DELETE 
    pub(crate) const XK_KP_Right: u64 = 0xff98;
    pub(crate) const XK_KP_Left: u64 = 0xff96;
    pub(crate) const XK_KP_Up: u64 = 0xff97;
    pub(crate) const XK_KP_Down: u64 = 0xff99;
    pub(crate) const XK_KP_Enter: u64 = 0xff8d; //<U+000D CARRIAGE RETURN>
    pub(crate) const XK_bracketleft: u64 = 0x005b; // U+005B LEFT SQUARE BRACKET 
    pub(crate) const XK_KP_Next: u64 = 0xff9b;
    pub(crate) const XK_KP_Delete: u64 = 0xff9f;
    pub(crate) const XK_KP_Home: u64 = 0xff95;
    pub(crate) const XK_KP_End: u64 = 0xff9c;
    pub(crate) const XK_KP_Prior: u64 = 0xff9a;
}
