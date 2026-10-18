# Güvenlik

Turnike, yurt ağına bağlanırken kullanıcı adı ve şifre kullandığı için güvenlik tarafında şu temel kuralları uygular:

- Kimlik ve parola bilgileri diske hiçbir zaman düz metin olarak yazılmaz. Bilgiler GNOME Keyring veya KWallet sistem anahtarlığında saklanır.
- Hata ayıklama çıktılarında veya terminal loglarında şifreler asla basılmaz, kimlik numaraları gibi özel bilgiler maskelenir.
- Ağ geçidiyle yapılan kimlik doğrulama, kota sorgulama ve şifre değiştirme işlemleri doğrudan HTTPS üzerinden yürütülür. Yalnızca ağ geçidinin araya girip girmediğini anlamak için yapılan portal tespiti denetimleri düz HTTP ile çalışır. Bu isteklerde hiçbir kullanıcı verisi taşınmaz.

## Desteklenen Sürümler

| Sürüm | Güvenlik Güncellemeleri |
| :--- | :--- |
| 0.1.x | Destekleniyor |
| main | Destekleniyor |

## Güvenlik Açığı Bildirimi

Güvenlik açıkları, herkese açık bir sorun kaydı açmak yerine GitHub deposundaki **Security Advisories** sekmesinden özel olarak bildirilir.

Bildirilen açıklar en kısa sürede incelenir, gerekli düzeltme hazırlanır ve açık herkese duyurulmadan önce düzeltilmiş yeni bir sürüm yayımlanır.

