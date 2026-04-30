//! Probes `/system/fonts/` for Roboto/Noto/Droid. Android has no fontconfig,
//! so base14 PostScript names need explicit mapping.

use std::fs;
use std::path::PathBuf;

use mupdf_sys::*;

use crate::font::Font;
use crate::system_font::SystemFontLoader;
use crate::CjkFontOrdering;

pub struct AndroidFontLoader;

impl SystemFontLoader for AndroidFontLoader {
    fn load_font(
        &self,
        name: &str,
        bold: bool,
        italic: bool,
        _needs_exact_metrics: bool,
    ) -> Option<Font> {
        let style = style_suffix(bold, italic);

        if eq_or_contains(name, "Helvetica") || eq_or_contains(name, "Arial") {
            return load_noto("Roboto", "", style, 0)
                .or_else(|| load_noto("NotoSans", "", style, 0))
                .or_else(|| load_noto("DroidSans", "", style, 0));
        }

        if eq_or_contains(name, "Times") || name.eq_ignore_ascii_case("Times-Roman") {
            return load_noto("NotoSerif", "", style, 0)
                .or_else(|| load_noto("RobotoSerif", "", style, 0))
                .or_else(|| load_noto("DroidSerif", "", style, 0));
        }

        if eq_or_contains(name, "Courier") {
            return load_noto("DroidSans", "Mono", "", 0)
                .or_else(|| load_noto("NotoSans", "Mono", "-Regular", 0));
        }

        if eq_or_contains(name, "Symbol") || eq_or_contains(name, "Dingbats") {
            return load_noto("NotoSans", "Symbols", "-Regular", 0)
                .or_else(|| load_noto("NotoSans", "Symbols2", "-Regular", 0));
        }

        None
    }

    fn load_cjk_font(
        &self,
        _name: &str,
        ordering: CjkFontOrdering,
        _serif: bool,
    ) -> Option<Font> {
        let lang = match ordering {
            CjkFontOrdering::AdobeCns => CjkLang::Tc,
            CjkFontOrdering::AdobeGb => CjkLang::Sc,
            CjkFontOrdering::AdobeJapan => CjkLang::Jp,
            CjkFontOrdering::AdobeKorea => CjkLang::Kr,
        };
        load_noto_cjk(lang)
    }

    fn load_fallback_font(
        &self,
        script: i32,
        language: i32,
        _serif: bool,
        _bold: bool,
        _italic: bool,
    ) -> Option<Font> {
        load_fallback(script, language)
    }
}

#[derive(Clone, Copy)]
enum CjkLang {
    Jp = 0,
    Kr = 1,
    Sc = 2,
    Tc = 3,
}

fn style_suffix(bold: bool, italic: bool) -> &'static str {
    match (bold, italic) {
        (true, true) => "-BoldItalic",
        (true, false) => "-Bold",
        (false, true) => "-Italic",
        (false, false) => "-Regular",
    }
}

fn eq_or_contains(name: &str, needle: &str) -> bool {
    name.eq_ignore_ascii_case(needle) || name.contains(needle)
}

fn load_noto(a: &str, b: &str, c: &str, idx: i32) -> Option<Font> {
    let stem = format!("/system/fonts/{a}{b}{c}");
    for ext in ["ttf", "otf", "ttc"] {
        let path = PathBuf::from(format!("{stem}.{ext}"));
        if let Ok(bytes) = fs::read(&path) {
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or(a);
            return Font::from_bytes_with_index(name, idx, &bytes).ok();
        }
    }
    None
}

fn load_noto_cjk(lang: CjkLang) -> Option<Font> {
    let idx = lang as i32;
    load_noto("NotoSerif", "CJK", "-Regular", idx)
        .or_else(|| load_noto("NotoSans", "CJK", "-Regular", idx))
        .or_else(|| load_noto("DroidSans", "Fallback", "", 0))
}

fn load_noto_arabic() -> Option<Font> {
    load_noto("Noto", "Naskh", "-Regular", 0)
        .or_else(|| load_noto("Noto", "NaskhArabic", "-Regular", 0))
        .or_else(|| load_noto("Droid", "Naskh", "-Regular", 0))
        .or_else(|| load_noto("NotoSerif", "Arabic", "-Regular", 0))
        .or_else(|| load_noto("NotoSans", "Arabic", "-Regular", 0))
        .or_else(|| load_noto("DroidSans", "Arabic", "-Regular", 0))
}

fn load_noto_try(stem: &str) -> Option<Font> {
    load_noto("NotoSerif", stem, "-Regular", 0)
        .or_else(|| load_noto("NotoSans", stem, "-Regular", 0))
        .or_else(|| load_noto("DroidSans", stem, "-Regular", 0))
}

fn load_fallback(script: i32, language: i32) -> Option<Font> {
    let script = script as u32;
    match script {
        UCDN_SCRIPT_HANGUL => load_noto_cjk(CjkLang::Kr),
        UCDN_SCRIPT_HIRAGANA | UCDN_SCRIPT_KATAKANA => load_noto_cjk(CjkLang::Jp),
        UCDN_SCRIPT_BOPOMOFO => load_noto_cjk(CjkLang::Tc),
        UCDN_SCRIPT_HAN => match language {
            l if l == FZ_LANG_ja as i32 => load_noto_cjk(CjkLang::Jp),
            l if l == FZ_LANG_ko as i32 => load_noto_cjk(CjkLang::Kr),
            l if l == FZ_LANG_zh_Hans as i32 => load_noto_cjk(CjkLang::Sc),
            _ => load_noto_cjk(CjkLang::Tc),
        },
        UCDN_SCRIPT_LATIN | UCDN_SCRIPT_GREEK | UCDN_SCRIPT_CYRILLIC => load_noto_try(""),
        UCDN_SCRIPT_ARABIC => load_noto_arabic(),
        _ => script_stem(script).and_then(load_noto_try),
    }
}

fn script_stem(script: u32) -> Option<&'static str> {
    let stem = match script {
        UCDN_SCRIPT_ARMENIAN => "Armenian",
        UCDN_SCRIPT_HEBREW => "Hebrew",
        UCDN_SCRIPT_SYRIAC => "Syriac",
        UCDN_SCRIPT_THAANA => "Thaana",
        UCDN_SCRIPT_DEVANAGARI => "Devanagari",
        UCDN_SCRIPT_BENGALI => "Bengali",
        UCDN_SCRIPT_GURMUKHI => "Gurmukhi",
        UCDN_SCRIPT_GUJARATI => "Gujarati",
        UCDN_SCRIPT_ORIYA => "Oriya",
        UCDN_SCRIPT_TAMIL => "Tamil",
        UCDN_SCRIPT_TELUGU => "Telugu",
        UCDN_SCRIPT_KANNADA => "Kannada",
        UCDN_SCRIPT_MALAYALAM => "Malayalam",
        UCDN_SCRIPT_SINHALA => "Sinhala",
        UCDN_SCRIPT_THAI => "Thai",
        UCDN_SCRIPT_LAO => "Lao",
        UCDN_SCRIPT_TIBETAN => "Tibetan",
        UCDN_SCRIPT_MYANMAR => "Myanmar",
        UCDN_SCRIPT_GEORGIAN => "Georgian",
        UCDN_SCRIPT_ETHIOPIC => "Ethiopic",
        UCDN_SCRIPT_CHEROKEE => "Cherokee",
        UCDN_SCRIPT_CANADIAN_ABORIGINAL => "CanadianAboriginal",
        UCDN_SCRIPT_OGHAM => "Ogham",
        UCDN_SCRIPT_RUNIC => "Runic",
        UCDN_SCRIPT_KHMER => "Khmer",
        UCDN_SCRIPT_MONGOLIAN => "Mongolian",
        UCDN_SCRIPT_YI => "Yi",
        UCDN_SCRIPT_OLD_ITALIC => "OldItalic",
        UCDN_SCRIPT_GOTHIC => "Gothic",
        UCDN_SCRIPT_DESERET => "Deseret",
        UCDN_SCRIPT_TAGALOG => "Tagalog",
        UCDN_SCRIPT_HANUNOO => "Hanunoo",
        UCDN_SCRIPT_BUHID => "Buhid",
        UCDN_SCRIPT_TAGBANWA => "Tagbanwa",
        UCDN_SCRIPT_LIMBU => "Limbu",
        UCDN_SCRIPT_TAI_LE => "TaiLe",
        UCDN_SCRIPT_LINEAR_B => "LinearB",
        UCDN_SCRIPT_UGARITIC => "Ugaritic",
        UCDN_SCRIPT_SHAVIAN => "Shavian",
        UCDN_SCRIPT_OSMANYA => "Osmanya",
        UCDN_SCRIPT_CYPRIOT => "Cypriot",
        UCDN_SCRIPT_BUGINESE => "Buginese",
        UCDN_SCRIPT_COPTIC => "Coptic",
        UCDN_SCRIPT_NEW_TAI_LUE => "NewTaiLue",
        UCDN_SCRIPT_GLAGOLITIC => "Glagolitic",
        UCDN_SCRIPT_TIFINAGH => "Tifinagh",
        UCDN_SCRIPT_SYLOTI_NAGRI => "SylotiNagri",
        UCDN_SCRIPT_OLD_PERSIAN => "OldPersian",
        UCDN_SCRIPT_KHAROSHTHI => "Kharoshthi",
        UCDN_SCRIPT_BALINESE => "Balinese",
        UCDN_SCRIPT_CUNEIFORM => "Cuneiform",
        UCDN_SCRIPT_PHOENICIAN => "Phoenician",
        UCDN_SCRIPT_PHAGS_PA => "PhagsPa",
        UCDN_SCRIPT_NKO => "NKo",
        UCDN_SCRIPT_SUNDANESE => "Sundanese",
        UCDN_SCRIPT_LEPCHA => "Lepcha",
        UCDN_SCRIPT_OL_CHIKI => "OlChiki",
        UCDN_SCRIPT_VAI => "Vai",
        UCDN_SCRIPT_SAURASHTRA => "Saurashtra",
        UCDN_SCRIPT_KAYAH_LI => "KayahLi",
        UCDN_SCRIPT_REJANG => "Rejang",
        UCDN_SCRIPT_LYCIAN => "Lycian",
        UCDN_SCRIPT_CARIAN => "Carian",
        UCDN_SCRIPT_LYDIAN => "Lydian",
        UCDN_SCRIPT_CHAM => "Cham",
        UCDN_SCRIPT_TAI_THAM => "TaiTham",
        UCDN_SCRIPT_TAI_VIET => "TaiViet",
        UCDN_SCRIPT_AVESTAN => "Avestan",
        UCDN_SCRIPT_EGYPTIAN_HIEROGLYPHS => "EgyptianHieroglyphs",
        UCDN_SCRIPT_SAMARITAN => "Samaritan",
        UCDN_SCRIPT_LISU => "Lisu",
        UCDN_SCRIPT_BAMUM => "Bamum",
        UCDN_SCRIPT_JAVANESE => "Javanese",
        UCDN_SCRIPT_MEETEI_MAYEK => "MeeteiMayek",
        UCDN_SCRIPT_IMPERIAL_ARAMAIC => "ImperialAramaic",
        UCDN_SCRIPT_OLD_SOUTH_ARABIAN => "OldSouthArabian",
        UCDN_SCRIPT_INSCRIPTIONAL_PARTHIAN => "InscriptionalParthian",
        UCDN_SCRIPT_INSCRIPTIONAL_PAHLAVI => "InscriptionalPahlavi",
        UCDN_SCRIPT_OLD_TURKIC => "OldTurkic",
        UCDN_SCRIPT_KAITHI => "Kaithi",
        UCDN_SCRIPT_BATAK => "Batak",
        UCDN_SCRIPT_BRAHMI => "Brahmi",
        UCDN_SCRIPT_MANDAIC => "Mandaic",
        UCDN_SCRIPT_CHAKMA => "Chakma",
        UCDN_SCRIPT_MIAO => "Miao",
        UCDN_SCRIPT_MEROITIC_CURSIVE | UCDN_SCRIPT_MEROITIC_HIEROGLYPHS => "Meroitic",
        UCDN_SCRIPT_SHARADA => "Sharada",
        UCDN_SCRIPT_SORA_SOMPENG => "SoraSompeng",
        UCDN_SCRIPT_TAKRI => "Takri",
        UCDN_SCRIPT_BASSA_VAH => "BassaVah",
        UCDN_SCRIPT_CAUCASIAN_ALBANIAN => "CaucasianAlbanian",
        UCDN_SCRIPT_DUPLOYAN => "Duployan",
        UCDN_SCRIPT_ELBASAN => "Elbasan",
        UCDN_SCRIPT_GRANTHA => "Grantha",
        UCDN_SCRIPT_KHOJKI => "Khojki",
        UCDN_SCRIPT_KHUDAWADI => "Khudawadi",
        UCDN_SCRIPT_LINEAR_A => "LinearA",
        UCDN_SCRIPT_MAHAJANI => "Mahajani",
        UCDN_SCRIPT_MANICHAEAN => "Manichaean",
        UCDN_SCRIPT_MENDE_KIKAKUI => "MendeKikakui",
        UCDN_SCRIPT_MODI => "Modi",
        UCDN_SCRIPT_MRO => "Mro",
        UCDN_SCRIPT_NABATAEAN => "Nabataean",
        UCDN_SCRIPT_OLD_NORTH_ARABIAN => "OldNorthArabian",
        UCDN_SCRIPT_OLD_PERMIC => "OldPermic",
        UCDN_SCRIPT_PAHAWH_HMONG => "PahawhHmong",
        UCDN_SCRIPT_PALMYRENE => "Palmyrene",
        UCDN_SCRIPT_PAU_CIN_HAU => "PauCinHau",
        UCDN_SCRIPT_PSALTER_PAHLAVI => "PsalterPahlavi",
        UCDN_SCRIPT_SIDDHAM => "Siddham",
        UCDN_SCRIPT_TIRHUTA => "Tirhuta",
        UCDN_SCRIPT_WARANG_CITI => "WarangCiti",
        UCDN_SCRIPT_AHOM => "Ahom",
        UCDN_SCRIPT_ANATOLIAN_HIEROGLYPHS => "AnatolianHieroglyphs",
        UCDN_SCRIPT_HATRAN => "Hatran",
        UCDN_SCRIPT_MULTANI => "Multani",
        UCDN_SCRIPT_OLD_HUNGARIAN => "OldHungarian",
        UCDN_SCRIPT_SIGNWRITING => "Signwriting",
        UCDN_SCRIPT_ADLAM => "Adlam",
        UCDN_SCRIPT_BHAIKSUKI => "Bhaiksuki",
        UCDN_SCRIPT_MARCHEN => "Marchen",
        UCDN_SCRIPT_NEWA => "Newa",
        UCDN_SCRIPT_OSAGE => "Osage",
        UCDN_SCRIPT_TANGUT => "Tangut",
        UCDN_SCRIPT_MASARAM_GONDI => "MasaramGondi",
        UCDN_SCRIPT_NUSHU => "Nushu",
        UCDN_SCRIPT_SOYOMBO => "Soyombo",
        UCDN_SCRIPT_ZANABAZAR_SQUARE => "ZanabazarSquare",
        UCDN_SCRIPT_DOGRA => "Dogra",
        UCDN_SCRIPT_GUNJALA_GONDI => "GunjalaGondi",
        UCDN_SCRIPT_HANIFI_ROHINGYA => "HanifiRohingya",
        UCDN_SCRIPT_MAKASAR => "Makasar",
        UCDN_SCRIPT_MEDEFAIDRIN => "Medefaidrin",
        UCDN_SCRIPT_OLD_SOGDIAN => "OldSogdian",
        UCDN_SCRIPT_SOGDIAN => "Sogdian",
        UCDN_SCRIPT_ELYMAIC => "Elymaic",
        UCDN_SCRIPT_NANDINAGARI => "Nandinagari",
        UCDN_SCRIPT_NYIAKENG_PUACHUE_HMONG => "NyiakengPuachueHmong",
        UCDN_SCRIPT_WANCHO => "Wancho",
        _ => return None,
    };
    Some(stem)
}
