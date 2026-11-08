Name:           turnike
Version:        0.1.2
Release:        1%{?dist}
Summary:        GSBWIFI için GTK4 masaüstü istemcisi

License:        AGPL-3.0-or-later
URL:            https://github.com/koder-cog/turnike
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  pkgconfig
BuildRequires:  pkgconfig(gtk4) >= 4.12
BuildRequires:  pkgconfig(libadwaita-1) >= 1.5
BuildRequires:  pkgconfig(libsecret-1)
BuildRequires:  pkgconfig(openssl)
BuildRequires:  desktop-file-utils
%if 0%{?suse_version}
BuildRequires:  appstream-glib
%else
BuildRequires:  libappstream-glib
%endif

%{!?_userunitdir: %define _userunitdir %{_prefix}/lib/systemd/user}
%{!?_metainfodir: %define _metainfodir %{_datadir}/metainfo}

%if 0%{?fedora} || 0%{?rhel}
Requires:       gtk4 >= 4.12
Requires:       libadwaita >= 1.5
Requires:       libsecret
Requires:       openssl-libs
%endif
Recommends:     NetworkManager

%description
GSBWIFI portalı için geliştirilmiş
GTK4 masaüstü istemcisi, arka plan servisi ve terminal aracı.
Tarayıcı açmaya gerek kalmadan portala bağlanmayı ve kalan kotayı takip etmeyi sağlar.

%prep
%autosetup -p1

%build
cargo build --release --workspace

%install
install -Dpm 0755 target/release/turnike %{buildroot}%{_bindir}/turnike
install -Dpm 0755 target/release/turnike-cli %{buildroot}%{_bindir}/turnike-cli
install -Dpm 0755 target/release/turnike-daemon %{buildroot}%{_bindir}/turnike-daemon

install -Dpm 0644 data/io.github.koder_cog.Turnike.desktop %{buildroot}%{_datadir}/applications/io.github.koder_cog.Turnike.desktop
install -Dpm 0644 data/io.github.koder_cog.Turnike.metainfo.xml %{buildroot}%{_metainfodir}/io.github.koder_cog.Turnike.metainfo.xml
install -Dpm 0644 data/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg
install -Dpm 0644 data/icons/hicolor/symbolic/apps/io.github.koder_cog.Turnike-symbolic.svg %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/io.github.koder_cog.Turnike-symbolic.svg
install -Dpm 0644 data/turnike-daemon.service %{buildroot}%{_userunitdir}/turnike-daemon.service

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/io.github.koder_cog.Turnike.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/io.github.koder_cog.Turnike.metainfo.xml || true
cargo test --workspace

%files
%license LICENSE
%doc README.md
%{_bindir}/turnike
%{_bindir}/turnike-cli
%{_bindir}/turnike-daemon
%{_datadir}/applications/io.github.koder_cog.Turnike.desktop
%{_metainfodir}/io.github.koder_cog.Turnike.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/io.github.koder_cog.Turnike.svg
%{_datadir}/icons/hicolor/symbolic/apps/io.github.koder_cog.Turnike-symbolic.svg
%{_userunitdir}/turnike-daemon.service

%changelog
* Sun Nov 08 2026 koder-cog <https://github.com/koder-cog> - 0.1.2-1
- Genel ve sosyal medya kotalari icin ayrik projeksiyon hesaplamasi

* Sun Nov 08 2026 koder-cog <https://github.com/koder-cog> - 0.1.1-1
- Fedora 45 ve Copr paketleme destegi

