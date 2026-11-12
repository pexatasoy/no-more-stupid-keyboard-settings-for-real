//! Windows yardımcıları: tek kopya, ikinci kopyadan "öne gel" sinyali, açılışta başlatma, simge.

use crate::dil::{ESKI_KIMLIK, KIMLIK};
use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Registry::*;
use windows::Win32::System::Threading::{CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, INFINITE};

const ACILIS_ANAHTARI: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub const SIMGE: &[u8] = include_bytes!("simge32.rgba");

fn ad(kimlik: &str, ek: &str) -> HSTRING {
    HSTRING::from(format!("Local\\{}.{}", kimlik, ek))
}

pub fn tek_ornek() -> bool {
    unsafe {
        let _ = CreateMutexW(None, true, &ad(KIMLIK, "TekOrnek"));
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

/// 1.0.x (Klavye Işık) tepside açıksa kapatır: ikisi aynı klavyeye yazıp çekişmesin.
/// Eski sürüm kendi adıyla bir kilit tutar; kilit varsa süreç adıyla sonlandırılır.
pub fn eski_surumu_kapat() {
    let acik = || unsafe {
        let h = CreateMutexW(None, false, &ad(ESKI_KIMLIK, "TekOrnek"));
        let var = GetLastError() == ERROR_ALREADY_EXISTS;
        if let Ok(h) = h {
            let _ = CloseHandle(h);
        }
        var
    };
    if !acik() {
        return;
    }
    // Dikkat: burada gunluk! YOK. Günlük klasor()'ü çağırır, o da eski klasörü taşımaya çalışır;
    // eski süreç dosyaları tutarken taşıma başarısız olur.
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill")
        .args(["/IM", &format!("{}.exe", ESKI_KIMLIK), "/F"])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status();
    for _ in 0..20 {
        if !acik() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // WebView2'nin alt süreçleri birkaç yüz ms daha klasörü tutabilir.
    std::thread::sleep(std::time::Duration::from_millis(600));
    crate::gunluk!("eski sürüm (KlavyeIsik.exe) kapatıldı");
}

/// İkinci kopya: açık olana "pencereni göster" der.
pub fn digerini_goster() {
    unsafe {
        if let Ok(h) = OpenEventW(EVENT_MODIFY_STATE, false, &ad(KIMLIK, "Goster")) {
            let _ = SetEvent(h);
        }
    }
}

/// İlk kopya: ikinci kopyanın sinyalini bekler, gelince `f` çağrılır.
pub fn goster_sinyalini_dinle(f: impl Fn() + Send + 'static) {
    let h = unsafe { CreateEventW(None, false, false, &ad(KIMLIK, "Goster")) };
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
    unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(ACILIS_ANAHTARI), &HSTRING::from(KIMLIK), RRF_RT_REG_SZ, None, None, None) == ERROR_SUCCESS }
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
                &HSTRING::from(KIMLIK),
                REG_SZ.0,
                Some(deger.as_ptr() as *const _),
                (deger.len() * 2) as u32,
            );
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(ACILIS_ANAHTARI), &HSTRING::from(KIMLIK));
        }
    }
}

/// Exe taşındıysa açılış kaydını yeni yola güncelle. 1.0.x'in "KlavyeIsik" kaydı varsa yenisine çevrilir.
pub fn acilis_yolunu_tazele() {
    let eski = unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(ACILIS_ANAHTARI), &HSTRING::from(ESKI_KIMLIK), RRF_RT_REG_SZ, None, None, None) == ERROR_SUCCESS };
    if eski {
        unsafe {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(ACILIS_ANAHTARI), &HSTRING::from(ESKI_KIMLIK));
        }
        acilis_ayarla(true);
        return;
    }
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
