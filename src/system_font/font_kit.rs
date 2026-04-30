//! Resolves PostScript names through the host font system via font-kit
//! (fontconfig on Linux, DirectWrite on Windows, CoreText on macOS).

use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;

use crate::font::Font;
use crate::system_font::SystemFontLoader;
use crate::CjkFontOrdering;

pub struct FontKitLoader;

impl SystemFontLoader for FontKitLoader {
    fn load_font(
        &self,
        name: &str,
        bold: bool,
        italic: bool,
        needs_exact_metrics: bool,
    ) -> Option<Font> {
        let font = lookup(name, bold, italic)?;
        if needs_exact_metrics
            && ((bold && !font.is_bold()) || (italic && !font.is_italic()))
        {
            return None;
        }
        Some(font)
    }

    fn load_cjk_font(&self, name: &str, ordering: CjkFontOrdering, serif: bool) -> Option<Font> {
        if let Some(font) = lookup(name, false, false) {
            return Some(font);
        }
        cjk_fallback(ordering, serif)
    }
}

fn lookup(name: &str, bold: bool, italic: bool) -> Option<Font> {
    let source = SystemSource::new();
    let handle = match source.select_by_postscript_name(name) {
        Ok(handle) => handle,
        Err(_) => {
            // Non-cumulative: each iteration overwrites `trimmed`, so a name
            // like "FooMTPS" only loses the trailing "PS".
            let mut trimmed = name;
            for suffix in ["MT", "PS", "IdentityH"] {
                if let Some(rest) = trimmed.strip_suffix(suffix) {
                    trimmed = rest;
                }
            }
            let mut properties = Properties::new();
            let properties = properties
                .weight(if bold { Weight::BOLD } else { Weight::NORMAL })
                .style(if italic { Style::Italic } else { Style::Normal });
            source
                .select_best_match(&[FamilyName::Title(trimmed.to_string())], properties)
                .ok()?
        }
    };
    handle_to_font(handle)
}

fn handle_to_font(handle: Handle) -> Option<Font> {
    let font_index = match handle {
        Handle::Path { font_index, .. } => font_index,
        Handle::Memory { font_index, .. } => font_index,
    };
    let loaded = handle.load().ok()?;
    let data = loaded.copy_font_data()?;
    Font::from_bytes_with_index(&loaded.family_name(), font_index as i32, &data).ok()
}

#[cfg(windows)]
fn cjk_fallback(ordering: CjkFontOrdering, serif: bool) -> Option<Font> {
    let candidates: &[&str] = match (serif, ordering) {
        (true, CjkFontOrdering::AdobeCns) => &["MingLiU"],
        (true, CjkFontOrdering::AdobeGb) => &["SimSun"],
        (true, CjkFontOrdering::AdobeJapan) => &["MS-Mincho"],
        (true, CjkFontOrdering::AdobeKorea) => &["Batang"],
        (false, CjkFontOrdering::AdobeCns) => &["DFKaiShu-SB-Estd-BF"],
        (false, CjkFontOrdering::AdobeGb) => &["KaiTi", "KaiTi_GB2312"],
        (false, CjkFontOrdering::AdobeJapan) => &["MS-Gothic"],
        (false, CjkFontOrdering::AdobeKorea) => &["Gulim"],
    };
    candidates.iter().find_map(|n| lookup(n, false, false))
}

#[cfg(not(windows))]
fn cjk_fallback(_ordering: CjkFontOrdering, _serif: bool) -> Option<Font> {
    None
}
