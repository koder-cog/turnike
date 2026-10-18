# Katkıda Bulunma Rehberi

Turnike projesine katkı sürecindeki temel kurallar ve iş akışı aşağıda özetlenmiştir.

## Paket Yapısı

Proje altı paketten oluşur:

- `turnike-types`: Ortak veri modelleri, parola doğrulama kuralları ve hata tipleri. Grafik veya ağ bağımlılığı almaz.
- `turnike-client`: Ağ geçidiyle konuşan HTTP istemcisi, çerez yönetimi ve HTML ayrıştırma motoru
- `turnike-secret`: GNOME Keyring veya KWallet sistem anahtarlığı entegrasyonu
- `turnike-daemon`: Ağ durumunu izleyen ve arka planda çalışan servis
- `turnike-cli`: Terminal aracı ve durum çubukları için JSON çıktısı sağlayan arayüz
- `turnike`: GTK4 ve Libadwaita masaüstü uygulaması

## Geliştirirken Dikkat Edilenler

Yurt ağlarında bağlantı kopmaları ve gecikmeler sık yaşandığı için arayüzün kilitlenmemesi önemlidir. Bu yüzden ağ operasyonları GTK ana döngüsünde değil, arka planda yürütülür ve arayüze kanal üzerinden aktarılır.

Kullanıcı adı, şifre ve T.C. kimlik numaraları hassas verilerdir. Parolalar hiçbir zaman diske düz metin olarak kaydedilmez. Hata ayıklama çıktılarında veya terminal loglarında şifrelerin ve kimlik numaralarının açıkça görünmemesine özen gösterilir.

Kod içi açıklamalarda kodun ne yaptığını kelimesi kelimesine tekrarlamak yerine, portalın getirdiği gariplikleri veya mimari tercihlerin nedenini açıklamak daha faydalıdır.

## Denetim Komutları

Değişiklikleri göndermeden önce kod biçimi, testler ve derleme şu komutlarla doğrulanabilir:

```bash
cargo fmt --all --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Commit Formatı

Commit başlıklarında [Conventional Commits formatı](https://www.conventionalcommits.org/tr/v1.0.0/) kullanılır. Tek satır ve İngilizce olması tercih edilse de zorunlu değildir:

- `feat(client): parse quota tables from primefaces html`
- `fix(gui): keep refresh button disabled while polling status`
- `docs(readme): update build dependencies`

## Katkı Gönderme

Depo çatallanıp yeni bir dalda çalışıldıktan sonra çekme isteği açılabilir. Testlerin ve linter denetiminin temiz geçmesi inceleme sürecini hızlandırır. Topluluk iletişimi için [Davranış Kuralları](CODE_OF_CONDUCT.md) geçerlidir.

