//! Klavye Işık — SteelSeries Apex 3 TKL için hafif ışık ayarı uygulaması (SteelSeries GG gerekmez).
//!
//! İş parçacıkları:
//! - motor.rs: klavyeyi bulur, ayarı basar, efektleri oynatır, takılınca yeniden basar
//! - ses.rs:   müzik efekti seçiliyken PC sesini dinler
//! - panel.rs: pencere (WebView2), tepsi simgesi, kısayollar (Windows)

#![cfg_attr(windows, windows_subsystem = "windows")]

mod ayarlar;
mod cihaz;
mod efekt;
mod komut;
mod motor;
mod ortak;
mod ses;
mod temalar;
#[cfg(not(windows))]
mod deneme_sunucu;
#[cfg(windows)]
mod panel;
#[cfg(windows)]
mod tepsi;
#[cfg(windows)]
mod yukleyici;

use ortak::Ortak;
use std::sync::Arc;

fn main() {
    #[cfg(windows)]
    if !tepsi::tek_ornek() {
        // Zaten açıksa onun penceresini öne getir.
        tepsi::digerini_goster();
        return;
    }
    let a = ayarlar::Ayarlar::yukle();
    gunluk!("Klavye Işık {} başlıyor", env!("CARGO_PKG_VERSION"));
    let o = Arc::new(Ortak::yeni(a));
    {
        let o = o.clone();
        std::thread::spawn(move || motor::calistir(o));
    }
    {
        let o = o.clone();
        std::thread::spawn(move || ses::calistir(o));
    }

    #[cfg(windows)]
    panel::calistir(o);
    #[cfg(not(windows))]
    deneme_sunucu::calistir(o);
}
