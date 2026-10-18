#!/bin/sh
set -e

# Turnike Evrensel Kurulum ve Temiz Kaldırma Aracı
#
# Kullanım:
#   sudo ./packaging/install.sh              (Sistem geneli: /usr/local)
#   ./packaging/install.sh --user            (Kullanıcı geneli: ~/.local)
#   ./packaging/install.sh --uninstall       (Tüm standart yollardan kaldırır)
#   ./packaging/install.sh --uninstall --purge (Yapılandırma ve önbelleği de siler)

PREFIX="/usr/local"
USER_MODE=0
UNINSTALL_MODE=0
PURGE_MODE=0

for arg in "$@"; do
    case "$arg" in
        --user)
            USER_MODE=1
            PREFIX="${HOME}/.local"
            ;;
        --uninstall)
            UNINSTALL_MODE=1
            ;;
        --purge)
            PURGE_MODE=1
            ;;
        --prefix=*)
            PREFIX="${arg#*=}"
            ;;
        -h|--help)
            echo "Turnike Evrensel Kurulum ve Kaldırma Aracı"
            echo ""
            echo "Kullanım:"
            echo "  $0 [SEÇENEKLER]"
            echo ""
            echo "Seçenekler:"
            echo "  --user         Sistem yerine mevcut kullanıcının dizinine (~/.local) kurar (root gerekmez)"
            echo "  --prefix=YOL   Özel kurulum dizini belirtir (varsayılan: /usr/local)"
            echo "  --uninstall    Turnike'yi sistemden temiz bir şekilde kaldırır"
            echo "  --purge        Kaldırma sırasında ~/.config/turnike ayarlarını ve önbelleği de siler"
            echo "  -h, --help     Bu yardım metnini görüntüler"
            exit 0
            ;;
        *)
            echo "Bilinmeyen argüman: $arg"
            echo "Yardım için: $0 --help"
            exit 1
            ;;
    esac
done

# --- KALDIRMA AKIŞI (UNINSTALL) ---
if [ "$UNINSTALL_MODE" -eq 1 ]; then
    echo "Turnike kaldırma işlemi başlatılıyor..."

    # Varsa aktif systemd kullanıcı servisini durdur
    if command -v systemctl >/dev/null 2>&1; then
        systemctl --user stop turnike-daemon.service >/dev/null 2>&1 || true
        systemctl --user disable turnike-daemon.service >/dev/null 2>&1 || true
    fi

    # Aranacak önek listesi: Kullanıcı elle prefix belirttiyse onu, yoksa tüm standart yolları tara
    TARGET_PREFIXES=""
    if [ "$USER_MODE" -eq 1 ]; then
        TARGET_PREFIXES="${HOME}/.local"
    elif [ "$PREFIX" != "/usr/local" ]; then
        TARGET_PREFIXES="$PREFIX"
    else
        TARGET_PREFIXES="/usr/local /usr ${HOME}/.local"
    fi

    REMOVED_ANY=0
    for p in $TARGET_PREFIXES; do
        MANIFEST="$p/share/turnike/install-manifest.txt"
        if [ -f "$MANIFEST" ]; then
            echo "Kurulum listesi bulundu: $MANIFEST"
            while IFS= read -r item || [ -n "$item" ]; do
                if [ -n "$item" ] && [ -e "$item" ]; then
                    rm -f "$item" 2>/dev/null || true
                    REMOVED_ANY=1
                fi
            done < "$MANIFEST"
            rm -rf "$p/share/turnike" 2>/dev/null || true
        fi

        # Manifest bulunamasa dahi standart dosyaları tara ve temizle
        FILES_TO_CHECK="
            $p/bin/turnike
            $p/bin/turnike-cli
            $p/bin/turnike-daemon
            $p/bin/turnike-uninstall
            $p/share/applications/io.github.koder_cog.Turnike.desktop
            $p/share/metainfo/io.github.koder_cog.Turnike.metainfo.xml
            $p/share/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg
            $p/share/icons/hicolor/symbolic/apps/io.github.koder_cog.Turnike-symbolic.svg
            $p/lib/systemd/user/turnike-daemon.service
            ${HOME}/.config/systemd/user/turnike-daemon.service
        "

        for f in $FILES_TO_CHECK; do
            if [ -e "$f" ]; then
                rm -f "$f" 2>/dev/null || true
                REMOVED_ANY=1
            fi
        done

        # Masaüstü ve simge önbelleklerini tazele
        if [ -d "$p/share/applications" ] && command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database "$p/share/applications" >/dev/null 2>&1 || true
        fi
        if [ -d "$p/share/icons/hicolor" ] && command -v gtk4-update-icon-cache >/dev/null 2>&1; then
            gtk4-update-icon-cache -q -t -f "$p/share/icons/hicolor" >/dev/null 2>&1 || true
        fi
    done

    # Kullanıcı ayarları temizliği (--purge)
    if [ "$PURGE_MODE" -eq 1 ]; then
        echo "Kullanıcı yapılandırması ve önbellek temizleniyor..."
        rm -rf "${HOME}/.config/turnike" 2>/dev/null || true
        rm -rf "${HOME}/.cache/turnike" 2>/dev/null || true
    fi

    if command -v systemctl >/dev/null 2>&1; then
        systemctl --user daemon-reload >/dev/null 2>&1 || true
    fi

    if [ "$REMOVED_ANY" -eq 1 ]; then
        echo "Turnike sisteminizden başarıyla kaldırıldı."
    else
        echo "Belirtilen konumlarda kurulu Turnike dosyası bulunamadı."
    fi
    exit 0
fi

# --- KURULUM AKIŞI (INSTALL) ---

# Dosya yollarını çözümle
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT=""
if [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
    REPO_ROOT="$SCRIPT_DIR"
elif [ -f "$SCRIPT_DIR/../Cargo.toml" ]; then
    REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
else
    REPO_ROOT="$PWD"
fi

DATA_DIR=""
if [ -d "$SCRIPT_DIR/data" ]; then
    DATA_DIR="$SCRIPT_DIR/data"
elif [ -d "$REPO_ROOT/data" ]; then
    DATA_DIR="$REPO_ROOT/data"
else
    echo "Hata: data/ dizini bulunamadı."
    exit 1
fi

# İkili dosyaları tespit et veya derle
BIN_SRC_TURNIKE=""
BIN_SRC_CLI=""
BIN_SRC_DAEMON=""

if [ -f "$SCRIPT_DIR/turnike" ] && [ -f "$SCRIPT_DIR/turnike-cli" ] && [ -f "$SCRIPT_DIR/turnike-daemon" ]; then
    # Yayınlanmış binary tarball içi
    BIN_SRC_TURNIKE="$SCRIPT_DIR/turnike"
    BIN_SRC_CLI="$SCRIPT_DIR/turnike-cli"
    BIN_SRC_DAEMON="$SCRIPT_DIR/turnike-daemon"
elif [ -f "$REPO_ROOT/target/release/turnike" ] && [ -f "$REPO_ROOT/target/release/turnike-cli" ] && [ -f "$REPO_ROOT/target/release/turnike-daemon" ]; then
    # target/release altında derlenmiş ikililer
    BIN_SRC_TURNIKE="$REPO_ROOT/target/release/turnike"
    BIN_SRC_CLI="$REPO_ROOT/target/release/turnike-cli"
    BIN_SRC_DAEMON="$REPO_ROOT/target/release/turnike-daemon"
else
    echo "target/release altında ikili dosyalar bulunamadı. Derleme başlatılıyor..."
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Hata: İkili dosyalar bulunamadı ve sistemde cargo derleyicisi yüklü değil."
        exit 1
    fi
    (cd "$REPO_ROOT" && cargo build --release --workspace)
    BIN_SRC_TURNIKE="$REPO_ROOT/target/release/turnike"
    BIN_SRC_CLI="$REPO_ROOT/target/release/turnike-cli"
    BIN_SRC_DAEMON="$REPO_ROOT/target/release/turnike-daemon"
fi

SYSTEMD_USER_DIR=""
if [ "$USER_MODE" -eq 1 ]; then
    SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
elif [ "$PREFIX" = "/usr" ] || [ "$PREFIX" = "/usr/local" ]; then
    SYSTEMD_USER_DIR="/usr/lib/systemd/user"
else
    SYSTEMD_USER_DIR="${PREFIX}/lib/systemd/user"
fi

BIN_DIR="${PREFIX}/bin"
APP_DIR="${PREFIX}/share/applications"
META_DIR="${PREFIX}/share/metainfo"
ICON_SCALABLE_DIR="${PREFIX}/share/icons/hicolor/scalable/apps"
ICON_SYMBOLIC_DIR="${PREFIX}/share/icons/hicolor/symbolic/apps"
TURNIKE_SHARE="${PREFIX}/share/turnike"

echo "Turnike kuruluyor -> ${PREFIX}"

mkdir -p "${BIN_DIR}"
mkdir -p "${APP_DIR}"
mkdir -p "${META_DIR}"
mkdir -p "${ICON_SCALABLE_DIR}"
mkdir -p "${ICON_SYMBOLIC_DIR}"
mkdir -p "${SYSTEMD_USER_DIR}"
mkdir -p "${TURNIKE_SHARE}"

# İkili dosyaları kur
install -m 755 "$BIN_SRC_TURNIKE" "${BIN_DIR}/turnike"
install -m 755 "$BIN_SRC_CLI" "${BIN_DIR}/turnike-cli"
install -m 755 "$BIN_SRC_DAEMON" "${BIN_DIR}/turnike-daemon"

# Masaüstü ve sistem dosyalarını kur
install -m 644 "${DATA_DIR}/io.github.koder_cog.Turnike.desktop" "${APP_DIR}/io.github.koder_cog.Turnike.desktop"
install -m 644 "${DATA_DIR}/io.github.koder_cog.Turnike.metainfo.xml" "${META_DIR}/io.github.koder_cog.Turnike.metainfo.xml"
install -m 644 "${DATA_DIR}/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg" "${ICON_SCALABLE_DIR}/io.github.koder_cog.Turnike.svg"
install -m 644 "${DATA_DIR}/icons/hicolor/symbolic/apps/io.github.koder_cog.Turnike-symbolic.svg" "${ICON_SYMBOLIC_DIR}/io.github.koder_cog.Turnike-symbolic.svg"
# Systemd servisinde ExecStart yolunu kurulan gerçek binary dizinine (BIN_DIR) göre dinamik ayarla
sed "s|^ExecStart=.*|ExecStart=${BIN_DIR}/turnike-daemon|" "${DATA_DIR}/turnike-daemon.service" > "${SYSTEMD_USER_DIR}/turnike-daemon.service"
chmod 644 "${SYSTEMD_USER_DIR}/turnike-daemon.service"

# Bağımsız kaldırıcı betiği (turnike-uninstall) oluştur
# Kullanıcı depoyu veya bu betiği silse dahi tek komutla her yerden kaldırabilmesini sağlar
cat << 'EOF' > "${BIN_DIR}/turnike-uninstall"
#!/bin/sh
set -e

echo "Turnike kaldırılıyor..."

# Varsa çalışan kullanıcı servisini durdur
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user stop turnike-daemon.service >/dev/null 2>&1 || true
    systemctl --user disable turnike-daemon.service >/dev/null 2>&1 || true
fi

SELF_DIR="$(cd "$(dirname "$0")" && pwd)"
PREFIX="$(cd "$SELF_DIR/.." && pwd)"
MANIFEST="$PREFIX/share/turnike/install-manifest.txt"

if [ -f "$MANIFEST" ]; then
    while IFS= read -r item || [ -n "$item" ]; do
        if [ -n "$item" ] && [ -e "$item" ]; then
            rm -f "$item" 2>/dev/null || true
        fi
    done < "$MANIFEST"
    rm -rf "$PREFIX/share/turnike" 2>/dev/null || true
fi

# Standart dosyaları doğrula ve sil
for f in \
    "$PREFIX/bin/turnike" \
    "$PREFIX/bin/turnike-cli" \
    "$PREFIX/bin/turnike-daemon" \
    "$PREFIX/share/applications/io.github.koder_cog.Turnike.desktop" \
    "$PREFIX/share/metainfo/io.github.koder_cog.Turnike.metainfo.xml" \
    "$PREFIX/share/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg" \
    "$PREFIX/share/icons/hicolor/symbolic/apps/io.github.koder_cog.Turnike-symbolic.svg" \
    "$PREFIX/lib/systemd/user/turnike-daemon.service" \
    "${HOME}/.config/systemd/user/turnike-daemon.service"; do
    [ -e "$f" ] && rm -f "$f" 2>/dev/null || true
done

if [ "$1" = "--purge" ]; then
    echo "Kullanıcı yapılandırması ve önbellek temizleniyor..."
    rm -rf "${HOME}/.config/turnike" 2>/dev/null || true
    rm -rf "${HOME}/.cache/turnike" 2>/dev/null || true
fi

if [ -d "$PREFIX/share/applications" ] && command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
fi
if [ -d "$PREFIX/share/icons/hicolor" ] && command -v gtk4-update-icon-cache >/dev/null 2>&1; then
    gtk4-update-icon-cache -q -t -f "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
fi
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload >/dev/null 2>&1 || true
fi

rm -f "$0" 2>/dev/null || true
echo "Turnike sisteminizden başarıyla kaldırıldı."
EOF

chmod +x "${BIN_DIR}/turnike-uninstall"

# Kurulan tüm dosyaların listesini manifest olarak kaydet
MANIFEST_FILE="${TURNIKE_SHARE}/install-manifest.txt"
cat << EOF > "$MANIFEST_FILE"
${BIN_DIR}/turnike
${BIN_DIR}/turnike-cli
${BIN_DIR}/turnike-daemon
${BIN_DIR}/turnike-uninstall
${APP_DIR}/io.github.koder_cog.Turnike.desktop
${META_DIR}/io.github.koder_cog.Turnike.metainfo.xml
${ICON_SCALABLE_DIR}/io.github.koder_cog.Turnike.svg
${ICON_SYMBOLIC_DIR}/io.github.koder_cog.Turnike-symbolic.svg
${SYSTEMD_USER_DIR}/turnike-daemon.service
EOF

# Masaüstü ve simge önbelleklerini güncelle
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APP_DIR}" >/dev/null 2>&1 || true
fi
if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
    gtk4-update-icon-cache -q -t -f "${PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload >/dev/null 2>&1 || true
fi

echo ""
echo "Turnike başarıyla kuruldu!"
echo "  - Grafik Arayüz: turnike"
echo "  - Komut Satırı: turnike-cli --help"
echo "  - Arka Plan Servisi: systemctl --user enable --now turnike-daemon.service"
echo "  - İleride Kaldırmak İçin: turnike-uninstall (veya $0 --uninstall)"
