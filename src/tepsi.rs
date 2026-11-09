//! Windows yardımcıları: tek kopya, ikinci kopyadan "öne gel" sinyali, açılışta başlatma, simge.

use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Registry::*;
use windows::Win32::System::Threading::{CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, INFINITE};

const ACILIS_ANAHTARI: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const ACILIS_DEGERI: &str = "KlavyeIsik";

pub const SIMGE: &[u8] = include_bytes!("simge32.rgba");

pub fn tek_ornek() -> bool {
    unsafe {
        let _ = CreateMutexW(None, true, w!("Local\\KlavyeIsik.TekOrnek"));
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

/// İkinci kopya: açık olana "pencereni göster" der.
pub fn digerini_goster() {
    unsafe {
        if let Ok(h) = OpenEventW(EVENT_MODIFY_STATE, false, w!("Local\\KlavyeIsik.Goster")) {
            let _ = SetEvent(h);
        }
    }
}

/// İlk kopya: ikinci kopyanın sinyalini bekler, gelince `f` çağrılır.
pub fn goster_sinyalini_dinle(f: impl Fn() + Send + 'static) {
    let h = unsafe { CreateEventW(None, false, false, w!("Local\\KlavyeIsik.Goster")) };
    let Ok(h) = h else { return };
    let ham = h.0 as usize;
    std::thread::spawn(move || {
        let h = HANDLE(ham as *mut _);
        loop {
            if unsafe { WaitForSingleObject(h, INFINITE) } == WAIT_OBJECT_0 {
                f();
            } else {
                break;
            }
        }
    });
}

pub fn acilista_mi() -> bool {
    unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(ACILIS_ANAHTARI), &HSTRING::from(ACILIS_DEGERI), RRF_RT_REG_SZ, None, None, None) == ERROR_SUCCESS }
}

pub fn acilis_ayarla(ac: bool) {
    unsafe {
        if ac {
            let Ok(yol) = std::env::current_exe() else { return };
            // --gizli: açılışta pencere açılmaz, yalnız tepside durur ve ışığı basar.
            let deger: Vec<u16> = format!("\"{}\" --gizli", yol.display()).encode_utf16().chain(std::iter::once(0)).collect();
            let _ = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &HSTRING::from(ACILIS_ANAHTARI),
                &HSTRING::from(ACILIS_DEGERI),
                REG_SZ.0,
                Some(deger.as_ptr() as *const _),
                (deger.len() * 2) as u32,
            );
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(ACILIS_ANAHTARI), &HSTRING::from(ACILIS_DEGERI));
        }
    }
}

/// Exe taşındıysa açılış kaydını yeni yola güncelle.
pub fn acilis_yolunu_tazele() {
    if acilista_mi() {
        acilis_ayarla(true);
    }
}

/// İlk açılışta panel "Windows açılınca başlasın mı?" diye sorar; cevap bir kez alınır.
pub fn acilis_soruldu_mu() -> bool {
    crate::ayarlar::klasor().join(".acilis-soruldu").exists()
}

pub fn acilis_soruldu() {
    let _ = std::fs::write(crate::ayarlar::klasor().join(".acilis-soruldu"), b"1");
}
