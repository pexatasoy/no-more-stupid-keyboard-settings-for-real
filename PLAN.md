# NoMoreStupidKeyboardSettingsForReal (eski adı: Klavye Işık)

Ad Efe'nin (2026-11-12). Açık kaynağa hazırlanıyor: README/LICENSE (GPL-2.0-or-later)/docs İngilizce; repo Efe "aç" deyince public olur.
Arayüz iki dilli (`src/dil.rs`): Windows dili Türkçeyse TR, değilse EN + ABD ANSI klavye çizimi; `NMSKS_DIL=tr|en` ezer.
1.1.0 ilk açılışta 1.0.x'ten taşır: `%APPDATA%\KlavyeIsik` → `%APPDATA%\NoMoreStupidKeyboardSettingsForReal`, açılış kaydı, açık kalan KlavyeIsik.exe kapatılır (`tepsi::eski_surumu_kapat`).

Efe'nin SteelSeries Apex 3 TKL klavyesi için SteelSeries GG'siz ışık uygulaması (Windows, tek .exe, ~1,4 MB).

## Durum (2026-11-09)
- 1.0.1 yayında (WebView2Loader.dll exe içine gömülü, bkz. src/yukleyici.rs): https://atasoy.dev/indir/klavye-isik-8185ed962678/KlavyeIsik.exe (`/var/www/atasoy-dev/indir/…`, Caddy'ye dokunmadan)
- 1.1.0 (yeni ad, iki dil, taşıma) aynı klasörde: …/NoMoreStupidKeyboardSettingsForReal.exe — gerçek Windows'ta taşıma henüz Efe'nin PC'sinde denenmedi
- Efe gerçek klavyesinde AKTİF kullanıyor (2026-11-12 teyit).

## Yapı
- Rust; `x86_64-pc-windows-gnu` sunucuda çapraz derlenir: `cargo build --release --target x86_64-pc-windows-gnu`
- `src/cihaz.rs` — USB HID protokolü (OpenRGB SteelSeriesApex8ZoneController'dan): VID 1038 PID 1622, arayüz 1, sayfa 0xFFC0;
  0x21 renk (8 bölge), 0x22 yerleşik gökkuşağı, 0x23 parlaklık 0..16, 0xA3 parlaklık sor
- `src/motor.rs` — klavyeyi bulur, 30 fps efekt, takılınca yeniden basar (klavyenin hafızası YOK)
- `src/efekt.rs` — efekt hesapları; `src/temalar.rs` hazır temalar; `src/ses.rs` WASAPI loopback + FFT (müzik)
- `src/panel.rs` + `src/panel.html` — tao/wry (WebView2) pencere, tepsi, Ctrl+Alt+K/J/L/M kısayolları
- Ayarlar: `%APPDATA%\NoMoreStupidKeyboardSettingsForReal\ayarlar.json`, günlük `gunluk.txt`

## Linux'ta deneme
`cargo test` (paket biçimi, efektler, ses bantları) · `cargo run --release` → http://127.0.0.1:4917/ (`NMSKS_PORT` ile değişir) (aynı arayüz, sahte klavye + sahte müzik).
Süreci PID'iyle kapat. ⚠️ `fuser -k <port>` KULLANMA: 4911 telefon-ai-isci'nin portu, 2026-11-12'de onu öldürdü (deneme portu bu yüzden 4917'ye taşındı).

## Bilinen riskler
- 0xA3 parlaklık okuma OpenRGB'de de "?" — motor ancak doğrulandıktan sonra güvenir.
- İmzasız exe: SmartScreen "Ek bilgi → Yine de çalıştır" ister.
