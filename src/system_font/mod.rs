//! Pluggable system-font loading. Implement [`SystemFontLoader`] and register
//! it via [`crate::Context::install_system_font_loader`].

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::OnceLock;

use mupdf_sys::*;

use crate::font::Font;
use crate::CjkFontOrdering;

#[cfg(all(feature = "system-fonts", not(target_arch = "wasm32")))]
pub mod font_kit;

#[cfg(feature = "android-fonts")]
pub mod android;

/// Trampolines may be invoked from any thread that holds a mupdf context, hence
/// `Send + Sync`. Default impls return `None` so callers only override what
/// they care about.
pub trait SystemFontLoader: Send + Sync + 'static {
    fn load_font(
        &self,
        name: &str,
        bold: bool,
        italic: bool,
        needs_exact_metrics: bool,
    ) -> Option<Font> {
        let _ = (name, bold, italic, needs_exact_metrics);
        None
    }

    fn load_cjk_font(&self, name: &str, ordering: CjkFontOrdering, serif: bool) -> Option<Font> {
        let _ = (name, ordering, serif);
        None
    }

    /// `script` is a `UCDN_SCRIPT_*` constant; `language` is an `FZ_LANG_*`
    /// constant (or `FZ_LANG_UNSET`).
    fn load_fallback_font(
        &self,
        script: i32,
        language: i32,
        serif: bool,
        bold: bool,
        italic: bool,
    ) -> Option<Font> {
        let _ = (script, language, serif, bold, italic);
        None
    }
}

static LOADER: OnceLock<Box<dyn SystemFontLoader>> = OnceLock::new();

pub(crate) fn install(loader: Box<dyn SystemFontLoader>) -> Result<(), Box<dyn SystemFontLoader>> {
    LOADER.set(loader)
}

/// mupdf wants a +1 reference back. We `fz_keep_font` to bump the count, then
/// let `Font::drop` decrement it — net delta of +1, transferred to mupdf.
unsafe fn forward_font(ctx: *mut fz_context, font: Font) -> *mut fz_font {
    let ptr = font.inner;
    fz_keep_font(ctx, ptr);
    ptr
}

pub(crate) unsafe extern "C" fn trampoline_load_font(
    ctx: *mut fz_context,
    name: *const c_char,
    bold: c_int,
    italic: c_int,
    needs_exact_metrics: c_int,
) -> *mut fz_font {
    let Some(loader) = LOADER.get() else {
        return ptr::null_mut();
    };
    let Ok(name) = CStr::from_ptr(name).to_str() else {
        return ptr::null_mut();
    };
    match loader.load_font(name, bold != 0, italic != 0, needs_exact_metrics != 0) {
        Some(font) => forward_font(ctx, font),
        None => ptr::null_mut(),
    }
}

pub(crate) unsafe extern "C" fn trampoline_load_cjk_font(
    ctx: *mut fz_context,
    name: *const c_char,
    ordering: c_int,
    serif: c_int,
) -> *mut fz_font {
    let Some(loader) = LOADER.get() else {
        return ptr::null_mut();
    };
    let Ok(name) = CStr::from_ptr(name).to_str() else {
        return ptr::null_mut();
    };
    let Ok(ordering) = CjkFontOrdering::try_from(ordering) else {
        return ptr::null_mut();
    };
    match loader.load_cjk_font(name, ordering, serif != 0) {
        Some(font) => forward_font(ctx, font),
        None => ptr::null_mut(),
    }
}

pub(crate) unsafe extern "C" fn trampoline_load_fallback_font(
    ctx: *mut fz_context,
    script: c_int,
    language: c_int,
    serif: c_int,
    bold: c_int,
    italic: c_int,
) -> *mut fz_font {
    let Some(loader) = LOADER.get() else {
        return ptr::null_mut();
    };
    match loader.load_fallback_font(script, language, serif != 0, bold != 0, italic != 0) {
        Some(font) => forward_font(ctx, font),
        None => ptr::null_mut(),
    }
}
