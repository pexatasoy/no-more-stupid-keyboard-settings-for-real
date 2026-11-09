//! WebView2Loader.dll'yi exe'nin içine gömer.
//!
//! mingw (windows-gnu) ile derleyince webview2-com, `CreateCoreWebView2EnvironmentWithOptions`'ı
//! WebView2Loader.dll'den içe aktarır: DLL exe'nin yanında yoksa Windows programı hiç açmaz.
//! Çözüm: o işlevin içe aktarma işaretçisini (`__imp_…`) burada kendimiz tanımlıyoruz; bağlayıcı yerel
//! tanımı bulunca DLL'yi içe aktarma tablosuna hiç koymaz. Çağrı gelince gömülü DLL
//! %APPDATA%\KlavyeIsik\WebView2Loader.dll'ye yazılır, oradan yüklenip asıl işleve aktarılır.

use std::ffi::c_void;
use std::sync::OnceLock;
use windows::core::{HSTRING, PCSTR};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

const DLL: &[u8] = include_bytes!("WebView2Loader.dll");
/// E_FAIL
const HATA: i32 = 0x80004005u32 as i32;

type Islev = unsafe extern "system" fn(*const u16, *const u16, *mut c_void, *mut c_void) -> i32;

fn asil() -> Option<Islev> {
    static ASIL: OnceLock<Option<usize>> = OnceLock::new();
    let p = *ASIL.get_or_init(|| {
        let yol = crate::ayarlar::klasor().join("WebView2Loader.dll");
        // Yoksa ya da başka sürümse (program güncellendi) yeniden yaz.
        if std::fs::read(&yol).map(|v| v != DLL).unwrap_or(true) {
            if let Err(e) = std::fs::write(&yol, DLL) {
                crate::gunluk!("WebView2Loader yazılamadı: {}", e);
            }
        }
        unsafe {
            let m = LoadLibraryW(&HSTRING::from(yol.as_os_str()))
                .map_err(|e| crate::gunluk!("WebView2Loader yüklenemedi: {}", e))
                .ok()?;
            GetProcAddress(m, PCSTR(b"CreateCoreWebView2EnvironmentWithOptions\0".as_ptr())).map(|f| f as usize)
        }
    });
    p.map(|f| unsafe { std::mem::transmute::<usize, Islev>(f) })
}

/// Düz addan yapılan başvurular için (bağlayıcı import kütüphanesini hiç çekmesin).
#[unsafe(no_mangle)]
pub unsafe extern "system" fn CreateCoreWebView2EnvironmentWithOptions(tarayici: *const u16, veri: *const u16, secenek: *mut c_void, isleyici: *mut c_void) -> i32 {
    match asil() {
        Some(f) => unsafe { f(tarayici, veri, secenek, isleyici) },
        None => HATA,
    }
}

#[unsafe(no_mangle)]
#[used]
pub static __imp_CreateCoreWebView2EnvironmentWithOptions: Islev = CreateCoreWebView2EnvironmentWithOptions;
