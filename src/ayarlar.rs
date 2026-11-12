//! Kalıcı ayarlar: %APPDATA%\NoMoreStupidKeyboardSettingsForReal\ayarlar.json
//! (Linux'ta deneme için ~/.config/no-more-stupid-keyboard-settings). 1.0.x'in %APPDATA%\KlavyeIsik klasörü ilk açılışta taşınır.
//!
//! Klavyenin kendi hafızası yok (fişten çekilince gökkuşağına döner), o yüzden asıl kayıt burası:
//! uygulama her açılışta ve klavye her takıldığında buradakini klavyeye yeniden basar.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const BOLGE: usize = 8;
/// Klavyenin donanım parlaklığı 0..=16 (Fn+F11/F12 ile de değişir).
pub const PARLAKLIK_EN_FAZLA: u8 = 16;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    pub fn hexten(s: &str) -> Option<Rgb> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let v = u32::from_str_radix(s, 16).ok()?;
        Some(Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }
}

/// Bir "sahne": hangi efekt, hangi renklerle, ne hızda. Temalar da birer sahnedir.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Sahne {
    pub efekt: String,
    /// Her bölgenin rengi (soldan sağa).
    #[serde(with = "renk_listesi")]
    pub renkler: [Rgb; BOLGE],
    /// Dalga/yıldız/ateş gibi efektlerin ikinci rengi.
    #[serde(with = "tek_renk")]
    pub vurgu: Rgb,
    /// 0.2 (yavaş) .. 3.0 (hızlı)
    pub hiz: f32,
    /// Hareket sağdan sola.
    pub ters: bool,
    /// Müzik efektlerinde duyarlılık 0.3..3.0
    pub hassasiyet: f32,
}

impl Default for Sahne {
    fn default() -> Self {
        crate::temalar::hazir()[0].sahne.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tema {
    pub ad: String,
    pub sahne: Sahne,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Ayarlar {
    pub sahne: Sahne,
    pub parlaklik: u8,
    /// Işıklar kapalıysa false (sahne korunur).
    pub acik: bool,
    pub kisayollar: bool,
    pub temalarim: Vec<Tema>,
    /// Kısayolla gezinirken en son seçilen tema (sıradaki/önceki için).
    pub son_tema: Option<String>,
}

impl Default for Ayarlar {
    fn default() -> Self {
        Ayarlar {
            sahne: Sahne::default(),
            parlaklik: PARLAKLIK_EN_FAZLA,
            acik: true,
            kisayollar: true,
            temalarim: Vec::new(),
            son_tema: None,
        }
    }
}

pub fn klasor() -> PathBuf {
    #[cfg(windows)]
    let kok = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    #[cfg(not(windows))]
    let kok = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")).unwrap_or_else(|| PathBuf::from("."));
    let k = kok.join(if cfg!(windows) { crate::dil::KIMLIK } else { "no-more-stupid-keyboard-settings" });
    static TASINDI: std::sync::Once = std::sync::Once::new();
    TASINDI.call_once(|| {
        let eski = kok.join(if cfg!(windows) { crate::dil::ESKI_KIMLIK } else { "klavye-isik" });
        if !k.exists() && eski.is_dir() && std::fs::rename(&eski, &k).is_err() {
            // Klasör kilitliyse (ör. WebView2 hâlâ tutuyor) yalnız önemli dosyaları kopyala:
            // renkler/temalar ve "açılışta sor" cevabı. Eski klasör yerinde kalır, zararı yok.
            let _ = std::fs::create_dir_all(&k);
            for f in ["ayarlar.json", ".acilis-soruldu"] {
                let _ = std::fs::copy(eski.join(f), k.join(f));
            }
        }
    });
    let _ = std::fs::create_dir_all(&k);
    k
}

impl Ayarlar {
    pub fn yukle() -> Ayarlar {
        let yol = klasor().join("ayarlar.json");
        let mut a = std::fs::read_to_string(&yol).ok().and_then(|s| serde_json::from_str::<Ayarlar>(&s).ok()).unwrap_or_default();
        a.duzelt();
        a
    }

    /// Elle bozulmuş ya da eski sürümden kalma değerleri sınırlar içine çeker.
    pub fn duzelt(&mut self) {
        self.parlaklik = self.parlaklik.min(PARLAKLIK_EN_FAZLA);
        let s = &mut self.sahne;
        s.hiz = if s.hiz.is_finite() { s.hiz.clamp(0.2, 3.0) } else { 1.0 };
        s.hassasiyet = if s.hassasiyet.is_finite() { s.hassasiyet.clamp(0.3, 3.0) } else { 1.0 };
        if !crate::efekt::EFEKTLER.iter().any(|e| e.0 == s.efekt) {
            s.efekt = "sabit".into();
        }
    }

    pub fn kaydet(&self) {
        let yol = klasor().join("ayarlar.json");
        let gecici = yol.with_extension("json.tmp");
        if let Ok(s) = serde_json::to_string_pretty(self) {
            if std::fs::write(&gecici, s).is_ok() {
                let _ = std::fs::rename(&gecici, &yol);
            }
        }
    }
}

/// Renkleri dosyada okunur "#rrggbb" olarak tutar.
mod tek_renk {
    use super::Rgb;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(r: &Rgb, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&r.hex())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Rgb, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Rgb::hexten(&s).unwrap_or(Rgb(255, 255, 255)))
    }
}

mod renk_listesi {
    use super::{Rgb, BOLGE};
    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeSeq};
    pub fn serialize<S: Serializer>(r: &[Rgb; BOLGE], s: S) -> Result<S::Ok, S::Error> {
        let mut q = s.serialize_seq(Some(BOLGE))?;
        for x in r {
            q.serialize_element(&x.hex())?;
        }
        q.end()
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[Rgb; BOLGE], D::Error> {
        let v = Vec::<String>::deserialize(d)?;
        let mut r = [Rgb(255, 255, 255); BOLGE];
        for (i, s) in v.iter().take(BOLGE).enumerate() {
            r[i] = Rgb::hexten(s).unwrap_or(Rgb(255, 255, 255));
        }
        Ok(r)
    }
}
