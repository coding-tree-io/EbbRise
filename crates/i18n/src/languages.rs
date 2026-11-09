//! Language names for the settings window, as upstream lists them in its preferences.

/// (code, name in that language), in upstream's order.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("ajp", "اللهجة الأردنية"),
    ("bg", "Български"),
    ("be", "Беларуская мова"),
    ("bn", "বাংলা"),
    ("ca", "Català"),
    ("cs", "Čeština"),
    ("da", "Dansk"),
    ("de", "Deutsch"),
    ("el", "Ελληνικά"),
    ("en", "English"),
    ("eo", "Esperanto"),
    ("es", "Español"),
    ("et", "Eesti keel"),
    ("fa", "فارسی"),
    ("fi", "Suomi"),
    ("fil", "Wikang Filipino"),
    ("fr", "Français"),
    ("gl", "Galego"),
    ("gu", "ગુજરાતી"),
    ("he", "עברית"),
    ("hi", "हिन्दी"),
    ("hu", "Magyar"),
    ("id", "Bahasa Indonesia"),
    ("it", "Italiano"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("lt", "Lietuvių"),
    ("lv", "Latviešu"),
    ("mk", "Македонски"),
    ("nb-NO", "Bokmål"),
    ("nl", "Nederlands"),
    ("np", "नेपाली"),
    ("pl", "Polski"),
    ("pt", "Português"),
    ("pt-BR", "Brasileiro"),
    ("pu", "ਪੰਜਾਬੀ"),
    ("ro", "Română"),
    ("ru", "Русский"),
    ("si", "සිංහල"),
    ("sk", "Slovenčina"),
    ("sl", "Slovenščina"),
    ("sr", "српски језик"),
    ("sv", "Svenska"),
    ("ta", "தமிழ்"),
    ("th", "ภาษาไทย"),
    ("tr", "Türkçe"),
    ("uk", "Українська"),
    ("vi", "Tiếng Việt"),
    ("yi", "ייִדיש"),
    ("zh-CN", "简体中文"),
    ("zh-TW", "正體中文"),
    ("arz", "مصرى"),
    ("cy", "Cymraeg"),
    ("eu", "Euskara"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locales::LOCALES;

    #[test]
    fn every_locale_has_exactly_one_name() {
        assert_eq!(LANGUAGES.len(), LOCALES.len());
        for (code, _) in LOCALES {
            assert_eq!(
                LANGUAGES.iter().filter(|(c, _)| c == code).count(),
                1,
                "{code}"
            );
        }
    }
}
