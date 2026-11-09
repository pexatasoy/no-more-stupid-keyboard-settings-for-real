//! Pencere (WebView2 içinde panel.html), tepsi simgesi ve genel kısayollar.
//!
//! Pencereyi kapatmak programı kapatmaz: tepside çalışmaya devam eder (klavyenin hafızası yok, ışığı
//! tutan bu program). Kapalı pencerenin WebView'ı 3 dakika sonra bellekten silinir; motor etkilenmez.

use crate::ayarlar::Sahne;
use crate::komut;
use crate::ortak::Ortak;
use crate::tepsi;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::platform::windows::WindowExtWindows;
use tao::window::{Window, WindowBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWM_WINDOW_CORNER_PREFERENCE};
use wry::{WebContext, WebView, WebViewBuilder};

const HTML: &str = include_str!("panel.html");
const GENISLIK: f64 = 1100.0;
const YUKSEKLIK: f64 = 720.0;

enum Olay {
    Ipc(String),
    Goster,
    Tepsi(TrayIconEvent),
    Menu(MenuEvent),
    Kisayol(GlobalHotKeyEvent),
}

struct Panel {
    pencere: Window,
    web: WebView,
}

struct Kisayollar {
    yonetici: Option<GlobalHotKeyManager>,
    kayitli: bool,
    sonraki: HotKey,
    onceki: HotKey,
    isik: HotKey,
    muzik: HotKey,
}

impl Kisayollar {
    fn yeni() -> Kisayollar {
        let m = Some(Modifiers::CONTROL | Modifiers::ALT);
        Kisayollar {
            yonetici: GlobalHotKeyManager::new().map_err(|e| crate::gunluk!("kısayol yöneticisi açılamadı: {}", e)).ok(),
            kayitli: false,
            sonraki: HotKey::new(m, Code::KeyK),
            onceki: HotKey::new(m, Code::KeyJ),
            isik: HotKey::new(m, Code::KeyL),
            muzik: HotKey::new(m, Code::KeyM),
        }
    }

    fn hepsi(&self) -> [HotKey; 4] {
        [self.sonraki, self.onceki, self.isik, self.muzik]
    }

    fn esitle(&mut self, istenen: bool) {
        let Some(y) = &self.yonetici else { return };
        if istenen == self.kayitli {
            return;
        }
        for h in self.hepsi() {
            let r = if istenen { y.register(h) } else { y.unregister(h) };
            if let Err(e) = r {
                crate::gunluk!("kısayol {}: {}", h.into_string(), e);
            }
        }
        self.kayitli = istenen;
    }
}

pub fn calistir(o: Arc<Ortak>) {
    tepsi::acilis_yolunu_tazele();
    let dongu = EventLoopBuilder::<Olay>::with_user_event().build();
    let vekil = dongu.create_proxy();

    // Tepsi, menü ve kısayol olayları döngüyü hemen uyandırsın (pencere gizliyken de).
    {
        let v = vekil.clone();
        TrayIconEvent::set_event_handler(Some(move |e| {
            let _ = v.send_event(Olay::Tepsi(e));
        }));
        let v = vekil.clone();
        MenuEvent::set_event_handler(Some(move |e| {
            let _ = v.send_event(Olay::Menu(e));
        }));
        let v = vekil.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |e| {
            let _ = v.send_event(Olay::Kisayol(e));
        }));
        let v = vekil.clone();
        tepsi::goster_sinyalini_dinle(move || {
            let _ = v.send_event(Olay::Goster);
        });
    }

    let menu = Menu::new();
    let ac = MenuItem::new("Klavye Işık’ı aç", true, None);
    let isik = MenuItem::new("Işıkları kapat", true, None);
    let sonraki = MenuItem::new("Sıradaki tema\tCtrl+Alt+K", true, None);
    let cikis = MenuItem::new("Çıkış", true, None);
    let _ = menu.append_items(&[&ac, &PredefinedMenuItem::separator(), &isik, &sonraki, &PredefinedMenuItem::separator(), &cikis]);
    let tepsi_simgesi = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("Klavye Işık")
        .with_icon(Icon::from_rgba(tepsi::SIMGE.to_vec(), 32, 32).expect("simge"))
        .build()
        .map_err(|e| crate::gunluk!("tepsi simgesi kurulamadı: {}", e))
        .ok();

    let mut kisayol = Kisayollar::yeni();
    let mut baglam = WebContext::new(Some(crate::ayarlar::klasor().join("webview")));
    let mut panel: Option<Panel> = None;
    let mut gorunur = false;
    let mut gizlenme: Option<Instant> = None;
    let mut son_kare = Instant::now();
    let mut gonderilen_surum = 0u64;
    let mut muzik_oncesi: Option<Sahne> = None;
    let mut ipucu = String::new();

    dongu.run(move |olay, hedef, akis| {
        *akis = if gorunur {
            ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(50))
        } else {
            ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(1000))
        };

        match olay {
            Event::NewEvents(StartCause::Init) => {
                if !std::env::args().any(|a| a == "--gizli") {
                    goster(hedef, &vekil, &mut baglam, &mut panel, &mut gorunur);
                }
            }
            Event::UserEvent(Olay::Goster) => goster(hedef, &vekil, &mut baglam, &mut panel, &mut gorunur),
            Event::UserEvent(Olay::Tepsi(TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. })) => {
                goster(hedef, &vekil, &mut baglam, &mut panel, &mut gorunur);
            }
            Event::UserEvent(Olay::Menu(e)) => {
                if e.id == ac.id() {
                    goster(hedef, &vekil, &mut baglam, &mut panel, &mut gorunur);
                } else if e.id == isik.id() {
                    let a = !o.ayarlar.lock().unwrap().acik;
                    o.degistir(|x| x.acik = a);
                } else if e.id == sonraki.id() {
                    komut::tema_gez(&o, 1);
                } else if e.id == cikis.id() {
                    cik(&o);
                    *akis = ControlFlow::Exit;
                    return;
                }
            }
            Event::UserEvent(Olay::Kisayol(e)) if e.state == HotKeyState::Pressed => {
                if e.id == kisayol.sonraki.id() {
                    komut::tema_gez(&o, 1);
                } else if e.id == kisayol.onceki.id() {
                    komut::tema_gez(&o, -1);
                } else if e.id == kisayol.isik.id() {
                    let a = !o.ayarlar.lock().unwrap().acik;
                    o.degistir(|x| x.acik = a);
                } else if e.id == kisayol.muzik.id() {
                    // Müzik moduna geç; yeniden basınca önceki sahneye dön.
                    let simdiki = o.ayarlar.lock().unwrap().sahne.clone();
                    if crate::efekt::muzik_mi(&simdiki.efekt) {
                        let geri = muzik_oncesi.take().unwrap_or_else(|| crate::temalar::hazir()[0].sahne.clone());
                        o.degistir(|a| a.sahne = geri);
                    } else {
                        muzik_oncesi = Some(simdiki);
                        komut::tema_uygula(&o, "Ekolayzır");
                    }
                }
            }
            Event::UserEvent(Olay::Ipc(s)) => {
                let Ok(v) = serde_json::from_str::<Value>(&s) else { return };
                match v["k"].as_str().unwrap_or("") {
                    "hazir" => gonderilen_surum = 0,
                    "gizle" => gizle(&mut panel, &mut gorunur, &mut gizlenme),
                    "kucult" => {
                        if let Some(p) = &panel {
                            p.pencere.set_minimized(true);
                        }
                    }
                    "surukle" => {
                        if let Some(p) = &panel {
                            let _ = p.pencere.drag_window();
                        }
                    }
                    "cikis" => {
                        cik(&o);
                        *akis = ControlFlow::Exit;
                        return;
                    }
                    "acilis" => {
                        tepsi::acilis_ayarla(v["v"].as_bool().unwrap_or(false));
                        tepsi::acilis_soruldu();
                        gonderilen_surum = 0;
                    }
                    "gunluk" => {
                        let _ = std::process::Command::new("notepad.exe").arg(crate::ayarlar::klasor().join("gunluk.txt")).spawn();
                    }
                    _ => komut::isle(&o, &v),
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => gizle(&mut panel, &mut gorunur, &mut gizlenme),
            _ => {}
        }

        let surum = o.surum.load(Ordering::SeqCst);
        if surum != gonderilen_surum {
            gonderilen_surum = surum;
            let (acik, kis) = {
                let a = o.ayarlar.lock().unwrap();
                (a.acik, a.kisayollar)
            };
            kisayol.esitle(kis);
            let _ = isik.set_text(if acik { "Işıkları kapat" } else { "Işıkları aç" });
            let yeni_ipucu = {
                let a = o.ayarlar.lock().unwrap();
                let bagli = if o.bagli.load(Ordering::SeqCst) { "" } else { " · klavye bulunamadı" };
                match (&a.son_tema, a.acik) {
                    (_, false) => format!("Klavye Işık — kapalı{}", bagli),
                    (Some(t), _) => format!("Klavye Işık — {}{}", t, bagli),
                    (None, _) => format!("Klavye Işık{}", bagli),
                }
            };
            if yeni_ipucu != ipucu {
                if let Some(t) = &tepsi_simgesi {
                    let _ = t.set_tooltip(Some(&yeni_ipucu));
                }
                ipucu = yeni_ipucu;
            }
            if let Some(p) = &panel {
                let d = komut::durum_json(&o, tepsi::acilista_mi(), !tepsi::acilis_soruldu_mu());
                let _ = p.web.evaluate_script(&format!("window.durum&&window.durum({})", d));
            }
        }

        if gorunur && son_kare.elapsed() >= Duration::from_millis(45) {
            son_kare = Instant::now();
            if let Some(p) = &panel {
                let _ = p.web.evaluate_script(&format!("window.kare&&window.kare({})", komut::kare_json(&o)));
            }
        }

        if !gorunur && panel.is_some() && gizlenme.map(|t| t.elapsed() > Duration::from_secs(180)).unwrap_or(false) {
            panel = None;
        }
    });
}

fn goster(hedef: &EventLoopWindowTarget<Olay>, vekil: &EventLoopProxy<Olay>, baglam: &mut WebContext, panel: &mut Option<Panel>, gorunur: &mut bool) {
    if panel.is_none() {
        *panel = olustur(hedef, vekil, baglam);
    }
    let Some(p) = panel.as_ref() else { return };
    p.pencere.set_minimized(false);
    p.pencere.set_visible(true);
    p.pencere.set_focus();
    let _ = p.web.evaluate_script("window.acildi&&window.acildi()");
    *gorunur = true;
}

fn gizle(panel: &mut Option<Panel>, gorunur: &mut bool, gizlenme: &mut Option<Instant>) {
    if let Some(p) = panel.as_ref() {
        p.pencere.set_visible(false);
    }
    *gorunur = false;
    *gizlenme = Some(Instant::now());
}

fn olustur(hedef: &EventLoopWindowTarget<Olay>, vekil: &EventLoopProxy<Olay>, baglam: &mut WebContext) -> Option<Panel> {
    let simge = tao::window::Icon::from_rgba(tepsi::SIMGE.to_vec(), 32, 32).ok();
    let pencere = WindowBuilder::new()
        .with_title("Klavye Işık")
        .with_decorations(false)
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(GENISLIK, YUKSEKLIK))
        .with_window_icon(simge)
        .with_visible(false)
        .build(hedef)
        .map_err(|e| crate::gunluk!("pencere açılamadı: {}", e))
        .ok()?;
    // Küçük ekranda (ör. 1080p + %150 ölçek) pencere sığmazsa her şeyi orantılı küçült.
    let mut olcek = 1.0f64;
    if let Some(ekran) = pencere.current_monitor() {
        let m = ekran.size().to_logical::<f64>(ekran.scale_factor());
        olcek = ((m.width - 40.0) / GENISLIK).min((m.height - 80.0) / YUKSEKLIK).clamp(0.6, 1.0);
        if olcek < 1.0 {
            pencere.set_inner_size(LogicalSize::new(GENISLIK * olcek, YUKSEKLIK * olcek));
        }
    }
    // Ekranın ortası.
    if let Some(ekran) = pencere.current_monitor() {
        let (e, b) = (ekran.size(), pencere.outer_size());
        let k = ekran.position();
        pencere.set_outer_position(tao::dpi::PhysicalPosition::new(
            k.x + (e.width as i32 - b.width as i32) / 2,
            k.y + (e.height as i32 - b.height as i32) / 2,
        ));
    }
    // Windows 11: yuvarlak köşe (Windows 10'da etkisiz).
    unsafe {
        let tercih: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            HWND(pencere.hwnd() as *mut _),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &tercih as *const _ as *const _,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
    let v = vekil.clone();
    let web = WebViewBuilder::new_with_web_context(baglam)
        .with_html(HTML)
        .with_background_color((11, 11, 18, 255))
        .with_devtools(false)
        .with_ipc_handler(move |istek| {
            let _ = v.send_event(Olay::Ipc(istek.body().clone()));
        })
        .build(&pencere);
    match web {
        Ok(web) => {
            if olcek < 1.0 {
                let _ = web.zoom(olcek);
            }
            Some(Panel { pencere, web })
        }
        Err(e) => {
            crate::gunluk!("WebView2 açılamadı: {}", e);
            use windows::core::w;
            use windows::Win32::UI::WindowsAndMessaging::*;
            unsafe {
                MessageBoxW(
                    None,
                    w!("Ayar ekranı için Microsoft WebView2 gerekiyor (Windows 11'de hazır gelir).\n\nAçılacak sayfadan “Evergreen Bootstrapper”ı kurup Klavye Işık'ı yeniden aç. Klavyenin ışığı bu arada çalışmaya devam eder."),
                    w!("Klavye Işık"),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            let _ = std::process::Command::new("explorer.exe").arg("https://developer.microsoft.com/microsoft-edge/webview2/").spawn();
            None
        }
    }
}

fn cik(o: &Arc<Ortak>) {
    o.calisiyor.store(false, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(150));
}
