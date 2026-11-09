//! Müzik efektleri için PC'nin sesini dinler: varsayılan çıkış aygıtından WASAPI loopback (sürücü gerekmez,
//! SyncYourPods'taki yakalamanın sadeleştirilmişi). Yalnız müzik efekti seçiliyken çalışır.
//!
//! Çıktı (ortak.ses): 8 frekans bandı (bastan tize, 0..1), bas seviyesi ve vuruş anı.

use crate::ayarlar::BOLGE;
use crate::efekt::Ses;
use crate::ortak::Ortak;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const N: usize = 2048;

pub fn calistir(o: Arc<Ortak>) {
    while o.calisiyor.load(Ordering::SeqCst) {
        if !o.ses_aktif.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(150));
            continue;
        }
        #[cfg(windows)]
        let sonuc = unsafe { win::yakala(&o) };
        #[cfg(not(windows))]
        let sonuc = deneme(&o);
        if let Err(e) = sonuc {
            crate::gunluk!("ses yakalama hatası: {}", e);
            std::thread::sleep(Duration::from_secs(1));
        }
        *o.ses.lock().unwrap() = Ses::default();
    }
}

/// Ses örneklerini toplar, yeterince birikince bantları hesaplar.
pub struct Cozumleyici {
    hiz: f32,
    tampon: Vec<f32>,
    /// Dairesel tamponda sıradaki yazma yeri.
    yer: usize,
    yazilan: usize,
    tepe: [f32; BOLGE],
    duzgun: [f32; BOLGE],
    bas_ort: f32,
    son_vurus: f32,
    zaman: f32,
    pencere: Vec<f32>,
}

impl Cozumleyici {
    pub fn yeni(hiz: u32) -> Cozumleyici {
        let pencere = (0..N).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos()).collect();
        Cozumleyici {
            hiz: hiz as f32,
            tampon: vec![0.0; N],
            yer: 0,
            yazilan: 0,
            tepe: [0.02; BOLGE],
            duzgun: [0.0; BOLGE],
            bas_ort: 0.0,
            son_vurus: 0.0,
            zaman: 0.0,
            pencere,
        }
    }

    /// Tek kanala indirilmiş örnekleri ekler; ~her 512 örnekte (48 kHz'de ~94 kez/sn) yeni sonuç verir.
    pub fn ekle(&mut self, ornek: &[f32], cikti: &mut impl FnMut(Ses)) {
        for &x in ornek {
            self.tampon[self.yer] = x;
            self.yer = (self.yer + 1) % N;
            self.yazilan += 1;
            if self.yazilan >= 512 {
                self.yazilan = 0;
                let s = self.hesapla(512.0 / self.hiz);
                cikti(s);
            }
        }
    }

    fn hesapla(&mut self, dt: f32) -> Ses {
        self.zaman += dt;
        // En eski örnek `yer`de: oradan başlayarak sırala, pencereyle çarp.
        let mut re: Vec<f32> = (0..N).map(|i| self.tampon[(self.yer + i) % N] * self.pencere[i]).collect();
        let mut im = vec![0.0f32; N];
        fft(&mut re, &mut im);
        // 40 Hz – 14 kHz arası 8 logaritmik bant.
        let (alt, ust) = (40.0f32, 14000.0f32);
        let mut ham = [0.0f32; BOLGE];
        for (b, h) in ham.iter_mut().enumerate() {
            let f0 = alt * (ust / alt).powf(b as f32 / BOLGE as f32);
            let f1 = alt * (ust / alt).powf((b + 1) as f32 / BOLGE as f32);
            let i0 = ((f0 * N as f32 / self.hiz) as usize).max(1);
            let i1 = ((f1 * N as f32 / self.hiz) as usize).max(i0 + 1).min(N / 2);
            let mut t = 0.0;
            for i in i0..i1 {
                t += re[i] * re[i] + im[i] * im[i];
            }
            // Tam ölçekli sinüs ≈ 1.0 olacak şekilde ölçekle.
            *h = (t / (i1 - i0) as f32).sqrt() / (N as f32 / 4.0);
        }
        let mut s = Ses::default();
        for b in 0..BOLGE {
            // Kendi kendine ayarlanan kazanç: her bant kendi son tepesine göre ölçülür (sessiz şarkıda da canlı),
            // ama tepe çok küçülüp gürültüyü büyütmesin diye taban var.
            self.tepe[b] = (self.tepe[b] * (1.0 - dt * 0.25)).max(ham[b]).max(0.004);
            let hedef = (ham[b] / self.tepe[b]).powf(1.4).min(1.0);
            let k = if hedef > self.duzgun[b] { 0.7 } else { (dt * 7.0).min(1.0) };
            self.duzgun[b] += (hedef - self.duzgun[b]) * k;
            s.bantlar[b] = self.duzgun[b];
        }
        s.bas = (self.duzgun[0] + self.duzgun[1]) / 2.0;
        // Vuruş: ham bas enerjisi son ~1 sn ortalamasının belirgin üstüne çıkınca (en sık 5/sn).
        let bas_ham = (ham[0] + ham[1]) / 2.0;
        s.vurus = bas_ham > self.bas_ort * 1.5 && bas_ham > 0.01 && self.zaman - self.son_vurus > 0.2;
        if s.vurus {
            self.son_vurus = self.zaman;
        }
        self.bas_ort += (bas_ham - self.bas_ort) * (dt * 1.5).min(1.0);
        s
    }
}

/// Yerinde radix-2 FFT (N ikinin kuvveti).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut boy = 2;
    while boy <= n {
        let aci = -std::f32::consts::TAU / boy as f32;
        let (wr, wi) = (aci.cos(), aci.sin());
        for bas in (0..n).step_by(boy) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..boy / 2 {
                let (a, b) = (bas + k, bas + k + boy / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let yeni = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = yeni;
            }
        }
        boy <<= 1;
    }
}

/// Linux'ta deneme: 120 BPM bas vuruşu + kayan bir tiz ton.
#[cfg(not(windows))]
fn deneme(o: &Arc<Ortak>) -> Result<(), String> {
    let hiz = 48000u32;
    let mut c = Cozumleyici::yeni(hiz);
    let mut n: u64 = 0;
    while o.ses_aktif.load(Ordering::SeqCst) {
        let mut parca = Vec::with_capacity(480);
        for _ in 0..480 {
            let t = n as f32 / hiz as f32;
            let vurus = (-((t % 0.5) * 18.0)).exp();
            let x = (t * 55.0 * std::f32::consts::TAU).sin() * vurus * 0.6
                + (t * (800.0 + 600.0 * (t * 0.3).sin()) * std::f32::consts::TAU).sin() * 0.15;
            parca.push(x);
            n += 1;
        }
        c.ekle(&parca, &mut |s| *o.ses.lock().unwrap() = s);
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[cfg(windows)]
mod win {
    use super::*;
    use windows::Win32::Foundation::WAIT_TIMEOUT;
    use windows::Win32::Media::Audio::*;
    use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
    use windows::Win32::Media::Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT};
    use windows::Win32::System::Com::*;
    use windows::Win32::System::Threading::*;

    unsafe fn aygit_kimligi(c: &IMMDevice) -> String {
        unsafe {
            match c.GetId() {
                Ok(p) => {
                    let s = p.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(p.0 as *const _));
                    s
                }
                Err(_) => String::new(),
            }
        }
    }

    pub unsafe fn yakala(o: &Arc<Ortak>) -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let numara: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let aygit = numara.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let kimlik = aygit_kimligi(&aygit);
            let istemci: IAudioClient = aygit.Activate(CLSCTX_ALL, None)?;
            let bp = istemci.GetMixFormat()?;
            let w = &*bp;
            let mut float = w.wFormatTag as u32 == WAVE_FORMAT_IEEE_FLOAT;
            if w.wFormatTag as u32 == WAVE_FORMAT_EXTENSIBLE {
                let x = &*(bp as *const WAVEFORMATEXTENSIBLE);
                float = std::ptr::addr_of!(x.SubFormat).read_unaligned() == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
            }
            let (hiz, kanal, bit) = (w.nSamplesPerSec, w.nChannels as usize, w.wBitsPerSample);
            crate::gunluk!("ses yakalama: {} Hz, {} kanal, {} bit{}", hiz, kanal, bit, if float { " float" } else { "" });
            istemci.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK, 200_000, 0, bp, None)?;
            let olay = CreateEventW(None, false, false, None)?;
            istemci.SetEventHandle(olay)?;
            let yakalayici: IAudioCaptureClient = istemci.GetService()?;
            istemci.Start()?;
            let mut coz = Cozumleyici::yeni(hiz);
            let mut tek: Vec<f32> = Vec::new();
            let mut son_bakis = std::time::Instant::now();
            let sonuc = (|| -> windows::core::Result<()> {
                while o.ses_aktif.load(Ordering::SeqCst) && o.calisiyor.load(Ordering::SeqCst) {
                    // Varsayılan çıkış değiştiyse (kulaklık takıldı vb.) baştan aç.
                    if son_bakis.elapsed() > Duration::from_secs(1) {
                        son_bakis = std::time::Instant::now();
                        if let Ok(a) = numara.GetDefaultAudioEndpoint(eRender, eConsole) {
                            if aygit_kimligi(&a) != kimlik {
                                return Ok(());
                            }
                        }
                    }
                    // Windows sessizlikte loopback olayı üretmez: zaman aşımında sessizlik say.
                    let zaman_asimi = WaitForSingleObject(olay, 30) == WAIT_TIMEOUT;
                    let mut okunan = 0usize;
                    loop {
                        let n = yakalayici.GetNextPacketSize()?;
                        if n == 0 {
                            break;
                        }
                        let mut veri: *mut u8 = std::ptr::null_mut();
                        let mut kare = 0u32;
                        let mut bayrak = 0u32;
                        yakalayici.GetBuffer(&mut veri, &mut kare, &mut bayrak, None, None)?;
                        let kare_n = kare as usize;
                        tek.clear();
                        if bayrak & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 || veri.is_null() || kanal == 0 {
                            tek.resize(kare_n, 0.0);
                        } else {
                            let toplam = kare_n * kanal;
                            let al = |i: usize| -> f32 {
                                if float && bit == 32 {
                                    *(veri as *const f32).add(i)
                                } else if bit == 16 {
                                    *(veri as *const i16).add(i) as f32 / 32768.0
                                } else if bit == 32 {
                                    *(veri as *const i32).add(i) as f32 / 2147483648.0
                                } else {
                                    0.0
                                }
                            };
                            let _ = toplam;
                            for f in 0..kare_n {
                                let mut t = 0.0;
                                for c in 0..kanal {
                                    t += al(f * kanal + c);
                                }
                                tek.push(t / kanal as f32);
                            }
                        }
                        yakalayici.ReleaseBuffer(kare)?;
                        coz.ekle(&tek, &mut |s| *o.ses.lock().unwrap() = s);
                        okunan += kare_n;
                    }
                    if zaman_asimi && okunan == 0 {
                        let sus = vec![0.0f32; (hiz as usize) * 30 / 1000];
                        coz.ekle(&sus, &mut |s| *o.ses.lock().unwrap() = s);
                    }
                }
                Ok(())
            })();
            let _ = istemci.Stop();
            CoTaskMemFree(Some(bp as *const _));
            sonuc
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bas_tonu_ilk_bantlari_doldurur() {
        let hiz = 48000;
        let mut c = Cozumleyici::yeni(hiz);
        let ornek: Vec<f32> = (0..hiz).map(|n| (n as f32 / hiz as f32 * 60.0 * std::f32::consts::TAU).sin() * 0.5).collect();
        let mut son = Ses::default();
        c.ekle(&ornek, &mut |s| son = s);
        assert!(son.bantlar[0] > 0.5, "{:?}", son.bantlar);
        assert!(son.bantlar[7] < 0.2, "{:?}", son.bantlar);
    }

    #[test]
    fn tiz_ton_son_bantlari_doldurur() {
        let hiz = 48000;
        let mut c = Cozumleyici::yeni(hiz);
        let ornek: Vec<f32> = (0..hiz).map(|n| (n as f32 / hiz as f32 * 9000.0 * std::f32::consts::TAU).sin() * 0.5).collect();
        let mut son = Ses::default();
        c.ekle(&ornek, &mut |s| son = s);
        assert!(son.bantlar[7] > 0.5, "{:?}", son.bantlar);
        assert!(son.bantlar[0] < 0.2, "{:?}", son.bantlar);
    }
}
