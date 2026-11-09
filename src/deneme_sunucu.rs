//! Yalnız Linux'ta, geliştirme denemesi için: panel.html'i gerçek motorla tarayıcıda açar.
//! http://127.0.0.1:4911/  — wry'nin `window.ipc.postMessage` köprüsü fetch ile taklit edilir.

use crate::komut;
use crate::ortak::Ortak;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

const HTML: &str = include_str!("panel.html");

const KOPRU: &str = r#"<script>
window.ipc={postMessage:s=>fetch('/ipc',{method:'POST',body:s}).then(()=>fetch('/durum')).then(r=>r.json()).then(d=>window.durum&&window.durum(d))};
setInterval(()=>fetch('/kare').then(r=>r.json()).then(d=>window.kare&&window.kare(d)).catch(()=>{}),50);
setInterval(()=>fetch('/durum').then(r=>r.json()).then(d=>window.durum&&window.durum(d)).catch(()=>{}),1500);
</script>"#;

pub fn calistir(o: Arc<Ortak>) {
    let port = std::env::var("KI_PORT").unwrap_or_else(|_| "4911".into());
    let dinle = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("port");
    eprintln!("deneme: http://127.0.0.1:{}/", port);
    for akis in dinle.incoming().flatten() {
        let o = o.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(&akis);
            let mut ilk = String::new();
            if r.read_line(&mut ilk).is_err() {
                return;
            }
            let mut uzunluk = 0usize;
            loop {
                let mut s = String::new();
                if r.read_line(&mut s).is_err() || s == "\r\n" || s.is_empty() {
                    break;
                }
                if let Some(x) = s.to_ascii_lowercase().strip_prefix("content-length:") {
                    uzunluk = x.trim().parse().unwrap_or(0);
                }
            }
            let mut govde = vec![0u8; uzunluk];
            let _ = r.read_exact(&mut govde);
            let yol = ilk.split_whitespace().nth(1).unwrap_or("/");
            let (tur, cevap) = match yol {
                "/ipc" => {
                    if let Ok(v) = serde_json::from_slice(&govde) {
                        komut::isle(&o, &v);
                    }
                    ("text/plain", "ok".to_string())
                }
                "/durum" => ("application/json", komut::durum_json(&o, false, std::env::var_os("KI_ILK").is_some())),
                "/kare" => ("application/json", komut::kare_json(&o)),
                _ => ("text/html; charset=utf-8", HTML.replacen("<head>", &format!("<head>{}", KOPRU), 1)),
            };
            let mut w = &akis;
            let _ = write!(w, "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", tur, cevap.len(), cevap);
        });
    }
}
