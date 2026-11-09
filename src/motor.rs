//! Motor: klavyeyi bulur, ayarı basar, efektleri saniyede 30 kare oynatır, fişten çekilip takılınca yeniden basar.

use crate::ayarlar::{Rgb, BOLGE};
use crate::cihaz::Klavye;
use crate::efekt::{self, Motor as EfektMotoru};
use crate::ortak::Ortak;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub fn calistir(o: Arc<Ortak>) {
    #[cfg(windows)]
    let mut arayici = crate::cihaz::win::Arayici::yeni();
    #[cfg(windows)]
    let mut bul = move || arayici.bul().map(|k| Box::new(k) as Box<dyn Klavye>);
    #[cfg(not(windows))]
    let mut bul = || Some(Box::new(Sahte::default()) as Box<dyn Klavye>);

    let mut klavye: Option<Box<dyn Klavye>> = None;
    let mut son_arama = Instant::now() - Duration::from_secs(10);
    let mut efm = EfektMotoru::default();
    let mut son_kare = Instant::now();
    let mut son_gonderim = Instant::now() - Duration::from_secs(10);
    let mut son_surum = 0u64;
    let mut son_parlaklik_okuma = Instant::now();
    let mut son_parlaklik_yazma = Instant::now();
    let mut gonderilen_parlaklik: Option<u8> = None;
    let mut donanim_basildi = false;
    // 0xA3 cevabına ancak bir kez gönderdiğimiz değeri doğru okuduktan sonra güvenilir;
    // değişiklik de üst üste iki okumada aynı çıkmalı (yanlış okuma klavyeyi karartmasın).
    let mut okuma_dogrulandi = false;
    let mut aday: Option<u8> = None;

    while o.calisiyor.load(Ordering::SeqCst) {
        // Klavye yoksa 2 saniyede bir ara.
        if klavye.is_none() {
            if son_arama.elapsed() >= Duration::from_secs(2) {
                son_arama = Instant::now();
                klavye = bul();
                if klavye.is_some() {
                    crate::gunluk!("klavye bağlandı");
                    o.bagli.store(true, Ordering::SeqCst);
                    o.surum.fetch_add(1, Ordering::SeqCst);
                    gonderilen_parlaklik = None;
                    donanim_basildi = false;
                    okuma_dogrulandi = false;
                    aday = None;
                    son_parlaklik_okuma = Instant::now();
                }
            }
            if klavye.is_none() {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
        }
        let k = klavye.as_mut().unwrap();

        let (sahne, parlaklik, acik) = {
            let a = o.ayarlar.lock().unwrap();
            (a.sahne.clone(), a.parlaklik, a.acik)
        };
        let surum = o.surum.load(Ordering::SeqCst);
        let yeni = surum != son_surum;
        son_surum = surum;
        let muzik = acik && efekt::muzik_mi(&sahne.efekt);
        o.ses_aktif.store(muzik, Ordering::SeqCst);

        let mut tamam = true;

        // Parlaklık: değiştiyse gönder.
        if gonderilen_parlaklik != Some(parlaklik) {
            tamam &= k.parlaklik(parlaklik);
            gonderilen_parlaklik = Some(parlaklik);
            son_parlaklik_yazma = Instant::now();
        }

        let dt = son_kare.elapsed().as_secs_f32();
        son_kare = Instant::now();
        let hareketli = acik && efekt::hareketli_mi(&sahne.efekt);

        if !acik {
            donanim_basildi = false;
            if yeni || son_gonderim.elapsed() >= Duration::from_secs(1) {
                let siyah = [Rgb::default(); BOLGE];
                tamam &= k.renk(&siyah);
                *o.kare.lock().unwrap() = siyah;
                son_gonderim = Instant::now();
            }
        } else if sahne.efekt == "donanim" {
            // Klavyenin kendi gökkuşağı: bir kez söylemek yeter, tekrar söylemek dalgayı baştan başlatır.
            if yeni || !donanim_basildi {
                tamam &= k.gokkusagi();
                donanim_basildi = true;
            }
            // Önizleme için aynı dalgayı yaklaşık çiz.
            let mut s = sahne.clone();
            s.efekt = "gokkusagi".into();
            s.hiz = 1.6;
            *o.kare.lock().unwrap() = efm.kare(&s, dt, &efekt::Ses::default());
        } else {
            donanim_basildi = false;
            let ses = *o.ses.lock().unwrap();
            let kare = efm.kare(&sahne, dt, &ses);
            if hareketli || yeni || son_gonderim.elapsed() >= Duration::from_secs(1) {
                tamam &= k.renk(&kare);
                son_gonderim = Instant::now();
            }
            *o.kare.lock().unwrap() = kare;
        }

        // Fn+F11/F12 ile klavyeden değiştirilen parlaklığı uygulamaya da yansıt.
        if tamam && son_parlaklik_okuma.elapsed() >= Duration::from_secs(2) && son_parlaklik_yazma.elapsed() >= Duration::from_secs(2) {
            son_parlaklik_okuma = Instant::now();
            match k.parlaklik_oku() {
                Some(p) if Some(p) == gonderilen_parlaklik => {
                    okuma_dogrulandi = true;
                    aday = None;
                }
                Some(p) if okuma_dogrulandi && aday == Some(p) => {
                    crate::gunluk!("klavyeden parlaklık değişti: {}", p);
                    gonderilen_parlaklik = Some(p);
                    aday = None;
                    o.degistir(|a| a.parlaklik = p);
                    son_surum = o.surum.load(Ordering::SeqCst);
                }
                Some(p) if okuma_dogrulandi => aday = Some(p),
                _ => {}
            }
        }

        if !tamam {
            crate::gunluk!("klavyeye yazılamadı, çıkarılmış olabilir");
            klavye = None;
            o.bagli.store(false, Ordering::SeqCst);
            o.surum.fetch_add(1, Ordering::SeqCst);
            continue;
        }

        std::thread::sleep(Duration::from_millis(if hareketli { 33 } else { 50 }));
    }
}

/// Linux deneme klavyesi: gelen kareleri saniyede bir günlüğe yazar.
#[cfg(not(windows))]
#[derive(Default)]
struct Sahte {
    son: Option<Instant>,
}

#[cfg(not(windows))]
impl Klavye for Sahte {
    fn renk(&mut self, c: &[Rgb; BOLGE]) -> bool {
        if self.son.map(|t| t.elapsed() > Duration::from_secs(1)).unwrap_or(true) {
            self.son = Some(Instant::now());
            crate::gunluk!("kare: {}", c.iter().map(|r| r.hex()).collect::<Vec<_>>().join(" "));
        }
        true
    }
    fn gokkusagi(&mut self) -> bool {
        crate::gunluk!("klavyenin kendi gökkuşağı");
        true
    }
    fn parlaklik(&mut self, p: u8) -> bool {
        crate::gunluk!("parlaklık {}", p);
        true
    }
    fn parlaklik_oku(&mut self) -> Option<u8> {
        None
    }
}
