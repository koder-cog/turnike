<div align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg" width="128" height="128" alt="Turnike İkonu" />
  <h1>Turnike</h1>
  <p><i>Rust ile 32 GB kotanın dayanılmaz hafifliği</i></p>
</div>

---

Turnike, KYK yurtlarındaki GSBWIFI portalı için geliştirilmiş bir GTK4 ve Libadwaita istemcisidir. Tarayıcı açmaya gerek kalmadan ağa bağlanmayı ve kalan kotayı takip etmeyi sağlar. Turnike projesinin Gençlik ve Spor Bakanlığı veya Kredi ve Yurtlar Genel Müdürlüğü ile resmi bir bağı yoktur.

## Özellikler

- İlk kayıt, şifre sıfırlama, şifre değiştirme ve oturum geçmişi
- Sistem anahtarlığında güvenli parola saklama
- Arka planda otomatik yeniden bağlanma ve terminal aracı
- Masaüstü bildirimleri ve ayarlanabilir kota uyarı eşiği
- Waybar ve durum çubukları için JSON çıktısı

## Kurulum

### Gereksinimler

i3, Sway veya Hyprland gibi pencere yöneticisi kullanılan ortamlarda kimlik bilgilerinin sistemde saklanabilmesi için gnome-keyring, keepassxc ya da kwallet gibi bir `org.freedesktop.secrets` sağlayıcısı gereklidir. GNOME ve KDE gibi masaüstü ortamlarında bunlar genellikle önyüklü olarak gelir.

### Ubuntu-Debian gibi dağıtımlar

[Releases](https://github.com/koder-cog/turnike/releases) sayfasından `.deb` paketi indirilerek:

```bash
sudo apt install ./turnike_*_amd64.deb

# Kaldırmak için
sudo apt remove turnike
```

### Fedora-openSUSE gibi dağıtımlar

COPR deposu eklenerek:

```bash
# Fedora Copr Deposu (Önerilen)
sudo dnf copr enable koder-cog/turnike
sudo dnf install turnike

# Kaldırmak için
sudo dnf remove turnike
```

Alternatif olarak [Releases](https://github.com/koder-cog/turnike/releases) sayfasından `.rpm` paketi indirilerek:

```bash
# Fedora / RHEL
sudo dnf install ./turnike-*.rpm

# openSUSE
# sudo zypper install ./turnike-*.rpm

# Kaldırmak için
sudo dnf remove turnike
```

### Arch Linux/CachyOS gibi dağıtımlar

```bash
git clone https://github.com/koder-cog/turnike.git
cd turnike/packaging/arch
makepkg -si

# Kaldırmak için
sudo pacman -R turnike
```

### Diğer Dağıtımlar

Depoyu klonlayarak veya [Releases](https://github.com/koder-cog/turnike/releases) sayfasındaki arşivden:

```bash
# Mevcut kullanıcı için
./install.sh --user

# Sistem geneli için
# sudo ./install.sh

# Kaldırmak için
./install.sh --uninstall
```

## Kullanım

### Grafik Arayüz

Uygulama menüsünden seçilebilir ya da terminalden başlatılabilir:

```bash
turnike
```

### Terminal Aracı

```bash
turnike-cli status           # Bağlantı ve oturum durumu
turnike-cli status --json    # Durum çubukları için JSON çıktısı
turnike-cli login            # Portala giriş
turnike-cli logout           # Çıkış
turnike-cli quota            # Kalan kotayı listele
turnike-cli ping             # Ping ölçümü
turnike-cli wifi radar       # Erişim noktası seçici
```

### Arka Plan Servisi

Uygulama üzerinden kimlik bilgileri kaydedilerek oturum açıldığında veya Ayarlar menüsünden arka plan servisi otomatik olarak etkinleştirilir. Servisi terminalden elle yönetmek için:

```bash
# Servisi başlatmak ve etkinleştirmek için
systemctl --user enable --now turnike-daemon.service

# Durumunu kontrol etmek için
systemctl --user status turnike-daemon.service
```

### Durum Çubuğu Entegrasyonu

Waybar yapılandırmasına şöyle bir modül eklenebilir:

```jsonc
"custom/turnike": {
    "format": " {text}",
    "return-type": "json",
    "exec": "turnike-cli status --json",
    "interval": 60,
    "on-click": "turnike",
    "tooltip": true
}
```

Ayrıntılı CSS ve modül örnekleri için [data/waybar-example.jsonc](data/waybar-example.jsonc) dosyasına bakabilirsiniz.

## Geliştiriciler İçin

### Bağımlılıklar

- Rust 1.85+
- GTK4 ve Libadwaita geliştirme paketleri
- Secret Service sağlayıcısı: `gnome-keyring`, `keepassxc` veya `kwallet` (`org.freedesktop.secrets`)
- OpenSSL ve libsecret geliştirme paketleri

Ubuntu / Debian:
```bash
sudo apt update
sudo apt install libgtk-4-dev libadwaita-1-dev libsecret-1-dev libssl-dev build-essential pkg-config
```

Fedora:
```bash
sudo dnf install gtk4-devel libadwaita-devel libsecret-devel openssl-devel gcc pkg-config
```

Arch Linux:
```bash
sudo pacman -S gtk4 libadwaita libsecret openssl base-devel pkgconf
```

### Derleme ve Test

```bash
cargo build --workspace
cargo build --release --workspace
cargo test --workspace
```


## Lisans

Bu proje [AGPL-3.0-or-later](LICENSE) ile lisanslanmıştır. Gayriresmi Türkçe çevirisi için [LICENSE_TR](LICENSE_TR) dosyasına bakabilirsiniz. Turnike marka hakları hakkında bilgi edinmek için [TRADEMARK](TRADEMARK.md) dosyasına bakabilirsiniz.