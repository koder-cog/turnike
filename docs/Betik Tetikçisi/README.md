# Betik Tetikçisi Kılavuzu

Turnike arka plan servisi turnike-daemon, ağ ve oturum durumundaki değişiklikleri algıladığında kullanıcının belirlediği harici kabuk komutlarını veya betiklerini otomatik olarak çalıştırabilir.

Bu mekanizma grafik arayüzde yer almaz. Güvenlik gerekçeleri ve sade bir kullanıcı deneyimi sağlamak amacıyla yalnızca yapılandırma dosyası üzerinden yönetilir.

---

## Yapılandırma Dosyası

Kancalar ev dizinindeki yapılandırma dosyasında tanımlanır:

```text
~/.config/turnike/config.json
```

Dosya mevcut değilse veya ilk kez oluşturuluyorsa hooks ve offpeak blokları aşağıdaki gibi eklenir:

```json
{
  "auto_login": true,
  "auto_preempt": true,
  "auto_metered": true,
  "hooks": {
    "on_login": "notify-send 'Turnike' 'GSBWIFI oturumu açıldı'",
    "on_logout": "notify-send 'Turnike' 'Oturum kapatıldı'",
    "on_quota_critical": "~/.config/turnike/scripts/quota_warning.sh",
    "on_throttled": "notify-send -u critical 'Turnike' 'Kota tükendi, hız kısıtlandı'",
    "on_offpeak_enter": "transmission-remote --start-all",
    "on_offpeak_exit": "transmission-remote --pause-all"
  },
  "offpeak": {
    "enabled": true,
    "start_hour": 2,
    "end_hour": 6,
    "max_rtt_ms": 50,
    "target_host": "1.1.1.1:53"
  }
}
```

---

## Desteklenen Olaylar

| Olay Adı | Tetiklenme Koşulu |
| :--- | :--- |
| `on_login` | GSBWIFI portalında oturum açıldığında çalışır |
| `on_logout` | Portal oturumu sonlandırıldığında çalışır |
| `on_quota_critical` | Kalan kota tanımlanan kritik eşiğin altına indiğinde çalışır |
| `on_throttled` | Yüksek hızlı kota tükenip 32 ile 64 kbps arası hız kısıtına girildiğinde çalışır |
| `on_offpeak_enter` | Belirlenen saat aralığında dış omurga gecikmesi eşik değerin altına indiğinde çalışır |
| `on_offpeak_exit` | Gece saat aralığından çıkıldığında veya ağ gecikmesi eşik değeri aştığında çalışır |

---

## Aktarılan Ortam Değişkenleri

Komut enjeksiyonu riskini engellemek amacıyla dinamik veriler doğrudan komut dizesine yapıştırılmaz. Bunun yerine alt sürece ortam değişkeni olarak aktarılır. Çalıştırılan betik içinde bu değişkenler doğrudan okunabilir:

| Değişken Adı | Açıklama | Örnek Değer |
| :--- | :--- | :--- |
| `TURNIKE_EVENT` | Tetiklenen olayın adı | `login`, `offpeak_enter`, `quota_critical` |
| `TURNIKE_RTT_MS` | Milisaniye cinsinden dış omurga TCP gecikmesi | `38` |
| `TURNIKE_REMAINING_MB` | Megabayt cinsinden kalan toplam kota | `8420.50` |
| `TURNIKE_TOTAL_MB` | Megabayt cinsinden toplam paket boyutu | `32768.00` |
| `TURNIKE_USERNAME` | Aktif oturumun kullanıcı adı veya T.C. kimlik numarası | `12345678901` |
| `TURNIKE_SSID` | Bağlı bulunulan kablosuz ağ adı | `GSBWIFI` |
| `TURNIKE_BSSID` | Bağlı bulunulan erişim noktasının MAC adresi | `00:1A:2B:3C:4D:5E` |

---

## Güvenlik ve Çalışma Kuralları

1. **Kullanıcı Yetkisi:** Komutlar arka planda `sh -c` aracılığıyla Turnike'yi çalıştıran kullanıcının yetkileriyle yürütülür. Yetkisiz dosya silme veya sisteme zarar verecek `rm -rf` türü komutlar kesinlikle yazılmamalıdır.
2. **On Beş Saniye Zaman Aşımı:** Kancaların ana servis döngüsünü kilitlemesini önlemek için her komut azami 15 saniye içinde tamamlanmak zorundadır. Bu süreyi aşan işlemler servis tarafından sonlandırılır. Uzun sürecek işler arka plana `&` veya `nohup` aracılığıyla devredilmelidir.
3. **İzole Yürütme:** Kancalar ana iş parçacığından bağımsız bir görev olarak çalıştırılır. Betiğin hata koduyla çıkması Turnike'nin çalışmasını engellemez, yalnızca günlüklere uyarı yazılır.

---

## Komut Satırından Test Etme

Tanımlanan bir kancayı veya betiği gerçek olayın gerçekleşmesini beklemeden test etmek için `turnike-cli` aracı kullanılabilir:

```bash
# Oturum açma kancasını test etme
turnike-cli hooks test login

# Gece indirme başlangıç kancasını test etme
turnike-cli hooks test offpeak_enter

# Kota kritik kancasını test etme
turnike-cli hooks test quota_critical
```

Geliştirme ortamında doğrudan çalıştırmak için:

```bash
cargo run -p turnike-cli -- hooks test login
```

Bu komut, yapılandırma dosyasındaki ilgili komutu test ortam değişkenleriyle tetikler ve başarı durumunu ekrana basar.

---

## Pratik Betik Örnekleri

### Örnek 1: Gece Saatlerinde İndirmeyi Otomatik Başlatma

Ağ rahatladığında `aria2` veya `transmission` istemcisini tetiklemek için `on_offpeak_enter` alanına komut tanımlanabilir:

```bash
# ~/.config/turnike/scripts/start_downloads.sh
#!/usr/bin/env bash

# Gecikme 50 ms altında ise ve gece saatindeysek indirmeleri başlat
if [ -n "$TURNIKE_RTT_MS" ]; then
    notify-send "Turnike" "Hat akıcı, gecikme $TURNIKE_RTT_MS ms. İndirmeler başlatılıyor."
    transmission-remote --start-all
fi
```

### Örnek 2: Kota Kritik Seviyeye İndiğinde Bildirim Gönderme

```bash
# ~/.config/turnike/scripts/quota_warning.sh
#!/usr/bin/env bash

notify-send -u critical "Kota Uyarısı" "Kalan kota kritik seviyeye indi: $TURNIKE_REMAINING_MB MB"
```
