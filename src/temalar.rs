//! Hazır temalar. Kullanıcının kaydettikleri ayarlar.json'da (`temalarim`).

use crate::ayarlar::{Rgb, Sahne, Tema, BOLGE};

fn r(h: &str) -> Rgb {
    Rgb::hexten(h).unwrap()
}

/// Birkaç renk arasında 8 bölgeye yumuşak geçiş.
pub fn gecis(duraklar: &[&str]) -> [Rgb; BOLGE] {
    let d: Vec<Rgb> = duraklar.iter().map(|h| r(h)).collect();
    let mut cikti = [Rgb::default(); BOLGE];
    for (i, c) in cikti.iter_mut().enumerate() {
        let t = i as f32 / (BOLGE - 1) as f32 * (d.len() - 1) as f32;
        let a = (t.floor() as usize).min(d.len() - 1);
        let b = (a + 1).min(d.len() - 1);
        let k = t - a as f32;
        let k = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
        *c = Rgb(k(d[a].0, d[b].0), k(d[a].1, d[b].1), k(d[a].2, d[b].2));
    }
    cikti
}

fn sira(desen: &[&str]) -> [Rgb; BOLGE] {
    let mut c = [Rgb::default(); BOLGE];
    for (i, x) in c.iter_mut().enumerate() {
        *x = r(desen[i % desen.len()]);
    }
    c
}

fn tema(ad: &str, efekt: &str, renkler: [Rgb; BOLGE], vurgu: &str, hiz: f32) -> Tema {
    Tema {
        ad: ad.into(),
        sahne: Sahne { efekt: efekt.into(), renkler, vurgu: r(vurgu), hiz, ters: false, hassasiyet: 1.0 },
    }
}

pub fn hazir() -> Vec<Tema> {
    vec![
        tema("Gün Batımı", "sabit", gecis(&["#5b2dff", "#ff2e93", "#ff7a1a", "#ffc21a"]), "#ffffff", 1.0),
        tema("Okyanus", "dalga", gecis(&["#0018ff", "#0070ff", "#00c8ff"]), "#b4fff6", 0.8),
        tema("Siberpunk", "sabit", gecis(&["#ff00c8", "#8a2bff", "#00e5ff"]), "#ffffff", 1.0),
        tema("Orman", "nefes", gecis(&["#0f7a00", "#3cff00", "#b6ff00"]), "#ffffff", 0.7),
        tema("Lav", "ates", sira(&["#ff1a00"]), "#ffb000", 1.0),
        tema("Buz", "nefes", gecis(&["#ffffff", "#8fd8ff", "#2a6bff"]), "#ffffff", 0.5),
        tema("Yıldızlı Gece", "yildiz", sira(&["#08105a"]), "#ffffff", 1.0),
        tema("Gökkuşağı", "gokkusagi", gecis(&["#ff0000", "#0000ff"]), "#ffffff", 1.0),
        tema("Gece Lambası", "sabit", sira(&["#5a2400"]), "#ff8a00", 1.0),
        tema("Ay Yıldız", "sabit", sira(&["#ff0000", "#ff0000", "#ff0000", "#ffffff", "#ffffff", "#ff0000", "#ff0000", "#ff0000"]), "#ffffff", 1.0),
        tema("Galatasaray", "sabit", sira(&["#ffb000", "#ffb000", "#e00000", "#e00000"]), "#ffffff", 1.0),
        tema("Fenerbahçe", "sabit", sira(&["#ffd000", "#ffd000", "#0a22ff", "#0a22ff"]), "#ffffff", 1.0),
        tema("Beşiktaş", "sabit", sira(&["#ffffff", "#ffffff", "#141414", "#141414"]), "#ffffff", 1.0),
        tema("Trabzonspor", "sabit", sira(&["#b0002e", "#b0002e", "#36b3ff", "#36b3ff"]), "#ffffff", 1.0),
        tema("Ekolayzır", "spektrum", gecis(&["#00ff2a", "#00ff2a", "#c8ff00", "#ffd000", "#ff6000", "#ff0010"]), "#ffffff", 1.0),
        tema("Parti", "nabiz", sira(&["#ff00c8", "#00e5ff", "#ffe600", "#7a00ff"]), "#ffffff", 1.0),
    ]
}
