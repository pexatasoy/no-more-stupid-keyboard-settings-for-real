//! Efektler: her karede 8 bölgenin rengini hesaplar. Donanımdan bağımsız, Linux'ta da denenebilir.
//!
//! Klavye yalnız "8 rengi şimdi göster" komutunu biliyor (bkz. cihaz.rs); hareketli her şey burada,
//! saniyede 30 kare hesaplanıp gönderilerek yapılır. Tek istisna "donanim": klavyenin kendi gökkuşağı.

use crate::ayarlar::{Rgb, Sahne, BOLGE};

/// (kimlik, ad, renkleri kullanıyor mu, müzik mi)
pub const EFEKTLER: &[(&str, &str, bool, bool)] = &[
    ("sabit", "Sabit", true, false),
    ("nefes", "Nefes", true, false),
    ("dalga", "Dalga", true, false),
    ("gokkusagi", "Gökkuşağı", false, false),
    ("dongu", "Renk Döngüsü", false, false),
    ("yildiz", "Yıldız Kayması", true, false),
    ("ates", "Ateş", true, false),
    ("spektrum", "Müzik · Spektrum", true, true),
    ("nabiz", "Müzik · Nabız", true, true),
    ("donanim", "Yerleşik Dalga", false, false),
];

pub fn muzik_mi(efekt: &str) -> bool {
    EFEKTLER.iter().any(|e| e.0 == efekt && e.3)
}

/// Hareketsiz efektler saniyede bir yeniden gönderilir (uykudan dönen klavye için), diğerleri her karede.
pub fn hareketli_mi(efekt: &str) -> bool {
    !matches!(efekt, "sabit" | "donanim")
}

/// Ses çözümlemesinden gelen anlık değerler (0..1).
#[derive(Clone, Copy, Default, Debug)]
pub struct Ses {
    pub bantlar: [f32; BOLGE],
    pub bas: f32,
    pub vurus: bool,
}

pub struct Motor {
    t: f32,
    seviye: [f32; BOLGE],
    hedef: [f32; BOLGE],
    tohum: u32,
    nabiz_sira: usize,
    nabiz_parlak: f32,
}

impl Default for Motor {
    fn default() -> Self {
        Motor { t: 0.0, seviye: [0.0; BOLGE], hedef: [0.0; BOLGE], tohum: 0x9E3779B9, nabiz_sira: 0, nabiz_parlak: 0.0 }
    }
}

fn karistir(a: Rgb, b: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round().clamp(0.0, 255.0) as u8;
    Rgb(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

fn olcekle(a: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    Rgb((a.0 as f32 * k).round() as u8, (a.1 as f32 * k).round() as u8, (a.2 as f32 * k).round() as u8)
}

/// Ton (0..1, döner) → tam doygun renk.
pub fn ton(h: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

impl Motor {
    fn rastgele(&mut self) -> f32 {
        // xorshift32: efekt için yeterli, bağımlılık gerektirmez.
        let mut x = self.tohum;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.tohum = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Bölgenin soldan sağa konumu 0..1 (ters ise sağdan sola).
    fn konum(s: &Sahne, i: usize) -> f32 {
        let x = i as f32 / (BOLGE - 1) as f32;
        if s.ters { 1.0 - x } else { x }
    }

    pub fn kare(&mut self, s: &Sahne, dt: f32, ses: &Ses) -> [Rgb; BOLGE] {
        let dt = dt.clamp(0.0, 0.25);
        self.t += dt * s.hiz;
        let t = self.t;
        let mut c = s.renkler;
        match s.efekt.as_str() {
            "nefes" => {
                // 4 saniyelik nefes, hiç tam sönmez.
                let k = 0.5 - 0.5 * (t * std::f32::consts::TAU / 4.0).cos();
                let k = 0.06 + 0.94 * k * k;
                for x in c.iter_mut() {
                    *x = olcekle(*x, k);
                }
            }
            "dalga" => {
                // Vurgu rengi bir uçtan öbürüne süzülür; uçlarda kısa bir duraklama payı var.
                let p = (t * 0.45).rem_euclid(1.6) - 0.3;
                for (i, x) in c.iter_mut().enumerate() {
                    let d = (Self::konum(s, i) - p) / 0.16;
                    *x = karistir(s.renkler[i], s.vurgu, (-d * d).exp());
                }
            }
            "gokkusagi" => {
                for (i, x) in c.iter_mut().enumerate() {
                    *x = ton(t * 0.12 - Self::konum(s, i) * 0.75);
                }
            }
            "dongu" => {
                let r = ton(t * 0.06);
                c = [r; BOLGE];
            }
            "yildiz" => {
                // Rastgele bölgeler vurgu rengiyle parlayıp yavaşça söner; zemin bölge renkleri.
                let olasilik = dt * s.hiz * 2.2;
                for i in 0..BOLGE {
                    self.seviye[i] = (self.seviye[i] - dt * (0.9 + s.hiz * 0.6)).max(0.0);
                }
                if self.rastgele() < olasilik {
                    let i = (self.rastgele() * BOLGE as f32) as usize % BOLGE;
                    self.seviye[i] = 1.0;
                }
                for (i, x) in c.iter_mut().enumerate() {
                    let k = self.seviye[i];
                    *x = karistir(s.renkler[i], s.vurgu, k * k * (3.0 - 2.0 * k));
                }
            }
            "ates" => {
                // Her bölge kendi hedefine doğru titrer: alevin dibi bölge rengi, tepesi vurgu.
                for i in 0..BOLGE {
                    if self.rastgele() < dt * 9.0 * s.hiz {
                        self.hedef[i] = self.rastgele().powf(1.6);
                    }
                    let h = (dt * 10.0 * s.hiz).min(1.0);
                    self.seviye[i] += (self.hedef[i] - self.seviye[i]) * h;
                }
                for (i, x) in c.iter_mut().enumerate() {
                    let k = self.seviye[i];
                    *x = olcekle(karistir(s.renkler[i], s.vurgu, k), 0.45 + 0.55 * k);
                }
            }
            "spektrum" => {
                // Soldan sağa: bastan tize. Sessizken hafif bir iz kalır ki klavye "kapandı" sanılmasın.
                for (i, x) in c.iter_mut().enumerate() {
                    let b = if s.ters { BOLGE - 1 - i } else { i };
                    *x = olcekle(s.renkler[i], 0.05 + 0.95 * ses.bantlar[b]);
                }
            }
            "nabiz" => {
                if ses.vurus {
                    self.nabiz_sira = (self.nabiz_sira + 1) % BOLGE;
                    self.nabiz_parlak = 1.0;
                }
                self.nabiz_parlak = (self.nabiz_parlak - dt * 2.5).max(ses.bas);
                let r = s.renkler[self.nabiz_sira];
                c = [olcekle(r, 0.06 + 0.94 * self.nabiz_parlak); BOLGE];
            }
            _ => {}
        }
        c
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn her_efekt_kare_uretir() {
        for (id, ..) in EFEKTLER {
            let mut s = crate::temalar::hazir()[0].sahne.clone();
            s.efekt = id.to_string();
            let mut m = Motor::default();
            let ses = Ses { bantlar: [0.5; BOLGE], bas: 0.7, vurus: true };
            for _ in 0..200 {
                m.kare(&s, 1.0 / 30.0, &ses);
            }
        }
    }

    #[test]
    fn sabit_renkleri_aynen_verir() {
        let s = crate::temalar::hazir()[0].sahne.clone();
        assert_eq!(Motor::default().kare(&s, 0.03, &Ses::default()), s.renkler);
    }

    #[test]
    fn gecis_uclari_tutar() {
        let g = crate::temalar::gecis(&["#000000", "#ffffff"]);
        assert_eq!(g[0], Rgb(0, 0, 0));
        assert_eq!(g[7], Rgb(255, 255, 255));
    }
}
