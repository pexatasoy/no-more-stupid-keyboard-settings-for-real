//! İş parçacıklarının paylaştığı durum: ayarlar, son gönderilen kare, bağlantı, ses çözümlemesi.

use crate::ayarlar::{Ayarlar, Rgb, BOLGE};
use crate::efekt::Ses;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

pub struct Ortak {
    pub ayarlar: Mutex<Ayarlar>,
    /// Ayar her değiştiğinde artar: motor yeni ayarı hemen basar, panel yeniden çizer.
    pub surum: AtomicU64,
    /// Klavyeye en son giden 8 renk (paneldeki canlı önizleme bunu gösterir).
    pub kare: Mutex<[Rgb; BOLGE]>,
    pub bagli: AtomicBool,
    pub ses: Mutex<Ses>,
    /// Müzik efekti seçiliyken true: ses yakalama yalnız o zaman çalışır.
    pub ses_aktif: AtomicBool,
    pub calisiyor: AtomicBool,
}

impl Ortak {
    pub fn yeni(a: Ayarlar) -> Ortak {
        Ortak {
            ayarlar: Mutex::new(a),
            surum: AtomicU64::new(1),
            kare: Mutex::new([Rgb::default(); BOLGE]),
            bagli: AtomicBool::new(false),
            ses: Mutex::new(Ses::default()),
            ses_aktif: AtomicBool::new(false),
            calisiyor: AtomicBool::new(true),
        }
    }

    /// Ayarı değiştir, kaydet, motora ve panele haber ver.
    pub fn degistir(&self, f: impl FnOnce(&mut Ayarlar)) {
        let mut a = self.ayarlar.lock().unwrap();
        f(&mut a);
        a.duzelt();
        a.kaydet();
        drop(a);
        self.surum.fetch_add(1, Ordering::SeqCst);
    }
}

/// Günlük: %APPDATA%\KlavyeIsik\gunluk.txt (1 MB'ı geçince baştan başlar).
#[macro_export]
macro_rules! gunluk {
    ($($a:tt)*) => {{
        let satir = format!($($a)*);
        $crate::ortak::gunluge_yaz(&satir);
    }};
}

pub fn gunluge_yaz(satir: &str) {
    use std::io::Write;
    let yol = crate::ayarlar::klasor().join("gunluk.txt");
    if std::fs::metadata(&yol).map(|m| m.len() > 1_000_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&yol);
    }
    let zaman = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&yol) {
        let _ = writeln!(f, "[{}] {}", zaman, satir);
    }
    #[cfg(not(windows))]
    eprintln!("{}", satir);
}
