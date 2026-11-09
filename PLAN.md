# Klavye Işık

Efe'nin SteelSeries Apex 3 TKL klavyesi için SteelSeries GG'siz ışık uygulaması (Windows, tek .exe, ~1,4 MB).

## Durum (2026-11-09)
- 1.0 yayında: https://atasoy.dev/indir/klavye-isik-8185ed962678/KlavyeIsik.exe (`/var/www/atasoy-dev/indir/…`, Caddy'ye dokunmadan)
- Gerçek klavyede HENÜZ denenmedi — ilk geri bildirim bekleniyor.

## Yapı
- Rust; `x86_64-pc-windows-gnu` sunucuda çapraz derlenir: `cargo build --release --target x86_64-pc-windows-gnu`
- `src/cihaz.rs` — USB HID protokolü (OpenRGB SteelSeriesApex8ZoneController'dan): VID 1038 PID 1622, arayüz 1, sayfa 0xFFC0;
  0x21 renk (8 bölge), 0x22 yerleşik gökkuşağı, 0x23 parlaklık 0..16, 0xA3 parlaklık sor
- `src/motor.rs` — klavyeyi bulur, 30 fps efekt, takılınca yeniden basar (klavyenin hafızası YOK)
- `src/efekt.rs` — efekt hesapları; `src/temalar.rs` hazır temalar; `src/ses.rs` WASAPI loopback + FFT (müzik)
- `src/panel.rs` + `src/panel.html` — tao/wry (WebView2) pencere, tepsi, Ctrl+Alt+K/J/L/M kısayolları
- Ayarlar: `%APPDATA%\KlavyeIsik\ayarlar.json`, günlük `gunluk.txt`

## Linux'ta deneme
`cargo test` (paket biçimi, efektler, ses bantları) · `cargo run --release` → http://127.0.0.1:4911/ (aynı arayüz, sahte klavye + sahte müzik).
Süreci `fuser -k 4911/tcp` ile kapat (`pkill -f` kabuğu da öldürür).

## Bilinen riskler
- 0xA3 parlaklık okuma OpenRGB'de de "?" — motor ancak doğrulandıktan sonra güvenir.
- İmzasız exe: SmartScreen "Ek bilgi → Yine de çalıştır" ister.
