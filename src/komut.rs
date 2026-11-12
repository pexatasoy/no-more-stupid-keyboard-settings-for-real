//! Panelden (panel.html) gelen komutlar ve panele giden durum. Pencereden bağımsız: Windows'ta wry,
//! Linux'ta deneme sunucusu aynı işlevleri kullanır.

use crate::ayarlar::{Sahne, Tema};
use crate::efekt::EFEKTLER;
use crate::ortak::Ortak;
use crate::temalar;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

/// `acilis`: Windows açılınca başlıyor mu; `acilis_sor`: ilk açılışta bunu sormak gerekiyor mu.
pub fn durum_json(o: &Ortak, acilis: bool, acilis_sor: bool) -> String {
    let a = o.ayarlar.lock().unwrap().clone();
    json!({
        "ayarlar": a,
        "hazirTemalar": temalar::hazir().into_iter().map(|t| { let e = crate::dil::tema_adi(&t.ad); let mut j = json!(t); j["etiket"] = json!(e); j }).collect::<Vec<_>>(),
        "efektler": EFEKTLER.iter().map(|e| json!({"id": e.0, "ad": crate::dil::efekt_adi(e.0), "renkli": e.2, "muzik": e.3})).collect::<Vec<_>>(),
        "dil": crate::dil::kod(),
        "uygulama": crate::dil::AD,
        "bagli": o.bagli.load(Ordering::SeqCst),
        "acilis": acilis,
        "acilisSor": acilis_sor,
        "surum": env!("CARGO_PKG_VERSION"),
        "windows": cfg!(windows),
    })
    .to_string()
}

pub fn kare_json(o: &Ortak) -> String {
    let k = *o.kare.lock().unwrap();
    let s = *o.ses.lock().unwrap();
    json!({
        "r": k.iter().map(|r| r.hex()).collect::<Vec<_>>(),
        "ses": s.bantlar.iter().map(|x| (x * 100.0).round() / 100.0).collect::<Vec<_>>(),
        "bagli": o.bagli.load(Ordering::SeqCst),
    })
    .to_string()
}

fn tum_temalar(o: &Ortak) -> Vec<Tema> {
    let mut t = temalar::hazir();
    t.extend(o.ayarlar.lock().unwrap().temalarim.iter().cloned());
    t
}

pub fn tema_uygula(o: &Ortak, ad: &str) {
    if let Some(t) = tum_temalar(o).into_iter().find(|t| t.ad == ad) {
        o.degistir(|a| {
            a.sahne = t.sahne;
            a.son_tema = Some(t.ad);
            a.acik = true;
        });
    }
}

/// Kısayolla sıradaki (+1) ya da önceki (-1) tema.
pub fn tema_gez(o: &Ortak, yon: i32) {
    let liste = tum_temalar(o);
    if liste.is_empty() {
        return;
    }
    let son = o.ayarlar.lock().unwrap().son_tema.clone();
    let i = son.and_then(|s| liste.iter().position(|t| t.ad == s)).map(|i| i as i32).unwrap_or(-1);
    let n = liste.len() as i32;
    let yeni = if i < 0 && yon < 0 { n - 1 } else { (i + yon).rem_euclid(n) };
    tema_uygula(o, &liste[yeni as usize].ad);
}

/// Genel komutlar. Pencereye ait olanlar (gizle, sürükle, açılış…) çağıran tarafta.
pub fn isle(o: &Ortak, v: &Value) {
    match v["k"].as_str().unwrap_or("") {
        "sahne" => {
            // Kısmi güncelleme: yalnız gelen alanlar değişir.
            let Some(yeni) = v.get("v").and_then(|x| x.as_object()) else { return };
            let mut simdiki = serde_json::to_value(&o.ayarlar.lock().unwrap().sahne).unwrap_or(Value::Null);
            if let Some(m) = simdiki.as_object_mut() {
                for (k, x) in yeni {
                    m.insert(k.clone(), x.clone());
                }
            }
            if let Ok(s) = serde_json::from_value::<Sahne>(simdiki) {
                o.degistir(|a| {
                    a.sahne = s;
                    a.son_tema = None;
                    a.acik = true;
                });
            }
        }
        "parlaklik" => {
            if let Some(p) = v["v"].as_u64() {
                o.degistir(|a| a.parlaklik = p as u8);
            }
        }
        "acik" => {
            let b = v["v"].as_bool().unwrap_or(true);
            o.degistir(|a| a.acik = b);
        }
        "kisayollar" => {
            let b = v["v"].as_bool().unwrap_or(true);
            o.degistir(|a| a.kisayollar = b);
        }
        "tema" => {
            if let Some(ad) = v["ad"].as_str() {
                tema_uygula(o, ad);
            }
        }
        "tema_kaydet" => {
            let ad = v["ad"].as_str().unwrap_or("").trim().chars().take(30).collect::<String>();
            if ad.is_empty() || temalar::hazir().iter().any(|t| t.ad == ad) {
                return;
            }
            o.degistir(|a| {
                let t = Tema { ad: ad.clone(), sahne: a.sahne.clone() };
                if let Some(x) = a.temalarim.iter_mut().find(|x| x.ad == ad) {
                    *x = t;
                } else {
                    a.temalarim.push(t);
                }
                a.son_tema = Some(ad);
            });
        }
        "tema_sil" => {
            if let Some(ad) = v["ad"].as_str() {
                let ad = ad.to_string();
                o.degistir(|a| {
                    a.temalarim.retain(|t| t.ad != ad);
                    if a.son_tema.as_deref() == Some(&ad) {
                        a.son_tema = None;
                    }
                });
            }
        }
        "tema_gez" => tema_gez(o, v["yon"].as_i64().unwrap_or(1) as i32),
        _ => {}
    }
}
