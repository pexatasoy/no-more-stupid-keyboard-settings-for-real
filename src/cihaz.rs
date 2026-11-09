//! SteelSeries Apex 3 TKL ile konuşma (USB HID, sürücü gerekmez).
//!
//! Protokol OpenRGB'nin SteelSeriesApex8ZoneController sürücüsünden (GPL-2.0, yalnız protokol bilgisi alındı):
//! - Aygıt: VID 0x1038, PID 0x1622, arayüz 1, kullanım sayfası 0xFFC0.
//! - Her paket 65 bayt: [0] rapor numarası (0), [1] komut, sonrası veri, kalanı sıfır.
//! - 0x21 0xFF R1 G1 B1 … R8 G8 B8 → 8 bölgenin rengi (0xFF: hepsini ayarla).
//! - 0x22 0xFF                    → klavyenin kendi gökkuşağı dalgası.
//! - 0x23 P                       → parlaklık, P = 0..16.
//! - 0xA3                         → parlaklığı sor; cevap [0xA3, P].
//!
//! Klavyenin kalıcı hafızası yok: fişten çekilince gökkuşağına döner. motor.rs bu yüzden takılınca yeniden basar.

use crate::ayarlar::{Rgb, BOLGE};

pub const VID: u16 = 0x1038;
pub const PID: u16 = 0x1622;
const PAKET: usize = 65;

pub trait Klavye {
    fn renk(&mut self, c: &[Rgb; BOLGE]) -> bool;
    fn gokkusagi(&mut self) -> bool;
    fn parlaklik(&mut self, p: u8) -> bool;
    fn parlaklik_oku(&mut self) -> Option<u8>;
}

fn paket(komut: u8, veri: &[u8]) -> [u8; PAKET] {
    let mut b = [0u8; PAKET];
    b[1] = komut;
    b[2..2 + veri.len()].copy_from_slice(veri);
    b
}

pub fn renk_paketi(c: &[Rgb; BOLGE]) -> [u8; PAKET] {
    let mut v = [0u8; 1 + BOLGE * 3];
    v[0] = 0xFF;
    for (i, r) in c.iter().enumerate() {
        v[1 + i * 3] = r.0;
        v[2 + i * 3] = r.1;
        v[3 + i * 3] = r.2;
    }
    paket(0x21, &v)
}

#[cfg(windows)]
pub mod win {
    use super::*;
    use hidapi::{HidApi, HidDevice};

    pub struct Apex {
        aygit: HidDevice,
    }

    pub struct Arayici {
        api: Option<HidApi>,
    }

    impl Arayici {
        pub fn yeni() -> Arayici {
            Arayici { api: HidApi::new().map_err(|e| crate::gunluk!("HID başlatılamadı: {}", e)).ok() }
        }

        /// Takılı bir Apex 3 TKL varsa açar. Klavyenin tuş arayüzüne dokunulmaz, yalnız ışık arayüzü.
        pub fn bul(&mut self) -> Option<Apex> {
            let api = self.api.as_mut()?;
            let _ = api.refresh_devices();
            let adaylar: Vec<_> = api
                .device_list()
                .filter(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0xFFC0)
                .collect();
            // Önce OpenRGB'nin kullandığı tam eşleşme (arayüz 1, kullanım 1), yoksa aynı sayfadaki herhangi biri.
            let secilen = adaylar
                .iter()
                .find(|d| d.interface_number() == 1 && d.usage() == 1)
                .or_else(|| adaylar.first())?;
            match secilen.open_device(api) {
                Ok(aygit) => Some(Apex { aygit }),
                Err(e) => {
                    crate::gunluk!("klavye açılamadı: {}", e);
                    None
                }
            }
        }
    }

    impl Apex {
        fn yaz(&mut self, b: &[u8; PAKET]) -> bool {
            self.aygit.write(b).is_ok()
        }
    }

    impl Klavye for Apex {
        fn renk(&mut self, c: &[Rgb; BOLGE]) -> bool {
            self.yaz(&renk_paketi(c))
        }
        fn gokkusagi(&mut self) -> bool {
            self.yaz(&paket(0x22, &[0xFF]))
        }
        fn parlaklik(&mut self, p: u8) -> bool {
            self.yaz(&paket(0x23, &[p.min(crate::ayarlar::PARLAKLIK_EN_FAZLA)]))
        }
        fn parlaklik_oku(&mut self) -> Option<u8> {
            if !self.yaz(&paket(0xA3, &[])) {
                return None;
            }
            let mut b = [0u8; PAKET];
            // Araya başka cevap girerse birkaç kez daha bak.
            for _ in 0..4 {
                match self.aygit.read_timeout(&mut b, 60) {
                    Ok(n) if n > 1 && b[0] == 0xA3 => return Some(b[1].min(crate::ayarlar::PARLAKLIK_EN_FAZLA)),
                    Ok(n) if n > 0 => continue,
                    _ => return None,
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn renk_paketi_openrgb_ile_ayni() {
        let mut c = [Rgb(0, 0, 0); BOLGE];
        c[0] = Rgb(1, 2, 3);
        c[7] = Rgb(250, 251, 252);
        let p = renk_paketi(&c);
        assert_eq!(p.len(), 65);
        assert_eq!(&p[0..6], &[0x00, 0x21, 0xFF, 1, 2, 3]);
        assert_eq!(&p[24..27], &[250, 251, 252]);
        assert!(p[27..].iter().all(|x| *x == 0));
    }

    #[test]
    fn parlaklik_paketi() {
        assert_eq!(&paket(0x23, &[16])[0..3], &[0x00, 0x23, 16]);
    }
}
