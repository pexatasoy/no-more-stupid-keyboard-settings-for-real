//! Uygulamanın adı ve dili.
//!
//! Arayüz iki dilli: Windows'un görüntü dili Türkçeyse Türkçe, değilse İngilizce.
//! `NMSKS_DIL=tr|en` ortam değişkeni bu seçimi ezer (deneme için).

use std::sync::OnceLock;

/// Görünen ad.
pub const AD: &str = "NoMoreStupidKeyboardSettingsForReal";
/// Ayar klasörü, açılış kaydı ve tek kopya kilidi için kimlik (Windows).
pub const KIMLIK: &str = "NoMoreStupidKeyboardSettingsForReal";
/// 1.0.x sürümlerinin kimliği: ayarlar ve açılış kaydı buradan taşınır.
pub const ESKI_KIMLIK: &str = "KlavyeIsik";

pub fn turkce() -> bool {
    static T: OnceLock<bool> = OnceLock::new();
    *T.get_or_init(|| {
        if let Ok(d) = std::env::var("NMSKS_DIL") {
            return d.starts_with("tr");
        }
        #[cfg(windows)]
        {
            // LANG_TURKISH = 0x1F (birincil dil kimliği: alt 10 bit)
            let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
            id & 0x3ff == 0x1f
        }
        #[cfg(not(windows))]
        {
            std::env::var("LANG").map(|l| l.starts_with("tr")).unwrap_or(false)
        }
    })
}

/// İki dilli metin: `t("Çıkış", "Quit")`.
pub fn t(tr: &'static str, en: &'static str) -> &'static str {
    if turkce() { tr } else { en }
}

pub fn kod() -> &'static str {
    t("tr", "en")
}

/// Efekt adı (kimlik → görünen ad).
pub fn efekt_adi(id: &str) -> &'static str {
    match id {
        "sabit" => t("Sabit", "Static"),
        "nefes" => t("Nefes", "Breathe"),
        "dalga" => t("Dalga", "Wave"),
        "gokkusagi" => t("Gökkuşağı", "Rainbow"),
        "dongu" => t("Renk Döngüsü", "Color Cycle"),
        "yildiz" => t("Yıldız Kayması", "Starfall"),
        "ates" => t("Ateş", "Fire"),
        "spektrum" => t("Müzik · Spektrum", "Music · Spectrum"),
        "nabiz" => t("Müzik · Nabız", "Music · Pulse"),
        "donanim" => t("Yerleşik Dalga", "Built-in Wave"),
        _ => "",
    }
}

/// Hazır temaların adı Türkçe saklanır (ayarlar.json'daki `son_tema` bununla eşleşir);
/// İngilizce arayüzde gösterirken çevrilir. Kullanıcının kendi temaları olduğu gibi kalır.
pub fn tema_adi(ad: &str) -> String {
    if turkce() {
        return ad.to_string();
    }
    match ad {
        "Gün Batımı" => "Sunset",
        "Okyanus" => "Ocean",
        "Siberpunk" => "Cyberpunk",
        "Orman" => "Forest",
        "Lav" => "Lava",
        "Buz" => "Ice",
        "Yıldızlı Gece" => "Starry Night",
        "Gökkuşağı" => "Rainbow",
        "Gece Lambası" => "Night Light",
        "Ay Yıldız" => "Crescent & Star",
        "Ekolayzır" => "Equalizer",
        "Parti" => "Party",
        x => return x.to_string(),
    }
    .to_string()
}
