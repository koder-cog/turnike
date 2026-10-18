use libadwaita::prelude::{ActionRowExt, PreferencesRowExt};
use turnike_types::PasswordRuleViolation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Tr,
    En,
}

impl Language {
    pub fn autodetect() -> Self {
        let lang_env = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();

        if lang_env.starts_with("en") {
            Self::En
        } else {
            Self::Tr
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Tr => "tr",
            Self::En => "en",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "tr" => Some(Self::Tr),
            "en" => Some(Self::En),
            _ => None,
        }
    }

    pub fn is_tr(&self) -> bool {
        matches!(self, Self::Tr)
    }

    pub fn index(&self) -> u32 {
        match self {
            Self::Tr => 0,
            Self::En => 1,
        }
    }

    pub fn from_index(idx: u32) -> Self {
        match idx {
            0 => Self::Tr,
            _ => Self::En,
        }
    }

    #[inline]
    pub fn texts(self) -> &'static Catalog {
        match self {
            Self::Tr => &TR_CATALOG,
            Self::En => &EN_CATALOG,
        }
    }

    pub fn violation_message(self, v: &PasswordRuleViolation) -> String {
        v.localized_message(self.is_tr())
    }

    pub fn portal_error_message(self, err: &turnike_types::PortalError) -> String {
        err.localized_message(self.is_tr())
    }

    pub fn format_recent_sessions(self, count: usize) -> String {
        match self {
            Self::Tr => format!("Son {count} oturum kaydı"),
            Self::En => format!("Last {count} session records"),
        }
    }

    pub fn format_quota_reset_countdown(self, raw: &str) -> String {
        let mut nums = Vec::new();
        for token in raw.split_whitespace() {
            if let Ok(n) = token.parse::<u64>() {
                nums.push(n);
            }
        }

        if nums.len() >= 3 {
            let (d, h, m) = (nums[0], nums[1], nums[2]);
            let mut parts = Vec::new();
            match self {
                Self::Tr => {
                    if d > 0 {
                        parts.push(format!("{d} gün"));
                    }
                    if h > 0 {
                        parts.push(format!("{h} sa"));
                    }
                    if m > 0 || (d == 0 && h == 0) {
                        parts.push(format!("{m} dk"));
                    }
                    return parts.join(" ");
                }
                Self::En => {
                    if d > 0 {
                        parts.push(format!("{d}d"));
                    }
                    if h > 0 {
                        parts.push(format!("{h}h"));
                    }
                    if m > 0 || (d == 0 && h == 0) {
                        parts.push(format!("{m}m"));
                    }
                    return parts.join(" ");
                }
            }
        }

        raw.to_string()
    }

    pub fn format_duration_seconds(self, total_seconds: u64) -> String {
        let d = total_seconds / 86400;
        let rem = total_seconds % 86400;
        let h = rem / 3600;
        let rem = rem % 3600;
        let m = rem / 60;
        let s = rem % 60;

        let mut parts = Vec::new();
        match self {
            Self::Tr => {
                if d > 0 {
                    parts.push(format!("{d} gün"));
                }
                if h > 0 {
                    parts.push(format!("{h} sa"));
                }
                if m > 0 || (d == 0 && h > 0) {
                    parts.push(format!("{m} dk"));
                }
                if d == 0 {
                    parts.push(format!("{s} sn"));
                }
            }
            Self::En => {
                if d > 0 {
                    parts.push(format!("{d}d"));
                }
                if h > 0 {
                    parts.push(format!("{h}h"));
                }
                if m > 0 || (d == 0 && h > 0) {
                    parts.push(format!("{m}m"));
                }
                if d == 0 {
                    parts.push(format!("{s}s"));
                }
            }
        }
        if parts.is_empty() {
            match self {
                Self::Tr => "0 sn".to_string(),
                Self::En => "0s".to_string(),
            }
        } else {
            parts.join(" ")
        }
    }

    pub fn format_portal_duration(self, raw: &str) -> String {
        let mut nums = Vec::new();
        for token in raw.split_whitespace() {
            if let Ok(n) = token.parse::<u64>() {
                nums.push(n);
            }
        }

        if nums.len() == 4 {
            let (d, h, m, s) = (nums[0], nums[1], nums[2], nums[3]);
            let total_secs = d * 86400 + h * 3600 + m * 60 + s;
            return self.format_duration_seconds(total_secs);
        }

        raw.to_string()
    }

    pub fn format_mb_or_gb(self, mb: f64) -> String {
        if mb >= 1024.0 {
            format!("{:.2} GB", mb / 1024.0)
        } else {
            format!("{:.0} MB", mb)
        }
    }

    pub fn format_foreign_wifi_banner(self, ssid: &str) -> String {
        match self {
            Self::Tr => format!("Bilgisayar '{ssid}' ağına bağlı"),
            Self::En => format!("Computer connected to '{ssid}'"),
        }
    }

    pub fn format_burn_rate_advice(self, proj: &turnike_types::QuotaProjection) -> String {
        if !proj.is_at_risk {
            match self {
                Self::Tr => "Bu hızla ay sonuna rahat yetiyor".to_string(),
                Self::En => "Usage comfortably lasts until month end".to_string(),
            }
        } else {
            let day_str = proj.projected_depletion_day.unwrap_or(20);
            match self {
                Self::Tr => format!("Bu hızla ayın {day_str}. gününde kota biter"),
                Self::En => format!("Quota runs out on day {day_str} at current pace"),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowText {
    pub title: &'static str,
    pub subtitle: &'static str,
}

impl RowText {
    pub const fn new(title: &'static str, subtitle: &'static str) -> Self {
        Self { title, subtitle }
    }
}

pub trait RowTextExt {
    fn apply_text(&self, text: &RowText);
}

impl<T: ActionRowExt + PreferencesRowExt> RowTextExt for T {
    fn apply_text(&self, text: &RowText) {
        self.set_title(text.title);
        self.set_subtitle(text.subtitle);
    }
}

pub struct AppTexts {
    pub title: &'static str,
    pub main_menu_tooltip: &'static str,
    pub preferences_menu: &'static str,
    pub shortcuts: &'static str,
    pub about: &'static str,
    pub about_developer: &'static str,
    pub about_comments: &'static str,
    pub about_source_code: &'static str,
}

pub struct StatusTexts {
    pub checking_connection: &'static str,
    pub connected: &'static str,
    pub not_logged_in: &'static str,
    pub no_connection: &'static str,
    pub login_failed: &'static str,
    pub foreign_network: &'static str,
}

pub struct NetworkTexts {
    pub foreign_wifi_toast: &'static str,
    pub disconnected_banner: &'static str,
}

pub struct AuthTexts {
    pub username_label: &'static str,
    pub password_label: &'static str,
    pub remember_me: &'static str,
    pub login_btn: &'static str,
    pub logout_btn: &'static str,
    pub logout_dialog_heading: &'static str,
    pub logout_dialog_body: &'static str,
    pub logout_dialog_cancel: &'static str,
    pub logout_dialog_confirm: &'static str,
    pub register_link: &'static str,
    pub forgot_password_link: &'static str,
}

pub struct QuotaTexts {
    pub title: &'static str,
    pub social: &'static str,
    pub general: &'static str,
    pub session_time: &'static str,
    pub wifi_ap: &'static str,
}

pub struct ProjectionTexts {
    pub title: &'static str,
    pub daily_safe_budget: &'static str,
    pub current_burn_rate: &'static str,
}

pub struct PreferencesTexts {
    pub title: &'static str,
    pub appearance_label: &'static str,
    pub appearance_options: [&'static str; 3],
    pub language_label: &'static str,
    pub service_group: &'static str,
    pub autostart: RowText,
    pub auto_login: RowText,
    pub auto_preempt: RowText,
    pub auto_metered: RowText,
    pub auto_logout_on_screen_lock: RowText,
    pub radar: RowText,
    pub quota_group: &'static str,
    pub notify_quota: RowText,
    pub threshold_mode_label: &'static str,
    pub threshold_mode_options: [&'static str; 2],
    pub threshold_value_label: &'static str,
}

pub struct PasswordTexts {
    pub change_title: &'static str,
    pub current_label: &'static str,
    pub temporary_label: &'static str,
    pub first_label: &'static str,
    pub new_label: &'static str,
    pub confirm_label: &'static str,
    pub rules_title: &'static str,
    pub rule_length: &'static str,
    pub rule_chars: &'static str,
    pub rule_forbidden_chars: &'static str,
    pub rule_forbidden_letters: &'static str,
    pub rule_no_username: &'static str,
    pub save_btn: &'static str,
    pub updated_toast: &'static str,
    pub mismatch_toast: &'static str,
}

pub struct RegisterTexts {
    pub title: &'static str,
    pub nationality_label: &'static str,
    pub nationality_tc: &'static str,
    pub nationality_foreigner: &'static str,
    pub user_type_label: &'static str,
    pub user_types: [&'static str; 6],
    pub verify_btn: &'static str,
    pub edevlet_info: &'static str,
}

pub struct ActionsTexts {
    pub title: &'static str,
    pub loading: &'static str,
    pub refresh_tooltip: &'static str,
    pub no_records: &'static str,
    pub summary_title: &'static str,
    pub download: &'static str,
    pub upload: &'static str,
}

pub struct HelpTexts {
    pub title: &'static str,
    pub sections: [(&'static str, &'static str); 4],
    pub shortcuts_list: [(&'static str, &'static str); 7],
}

pub struct Catalog {
    pub app: AppTexts,
    pub status: StatusTexts,
    pub network: NetworkTexts,
    pub auth: AuthTexts,
    pub quota: QuotaTexts,
    pub projection: ProjectionTexts,
    pub preferences: PreferencesTexts,
    pub password: PasswordTexts,
    pub register: RegisterTexts,
    pub actions: ActionsTexts,
    pub help: HelpTexts,
}

pub static TR_CATALOG: Catalog = Catalog {
    app: AppTexts {
        title: "Turnike",
        main_menu_tooltip: "Ana Menü",
        preferences_menu: "Tercihler",
        shortcuts: "Klavye Kısayolları",
        about: "Turnike Hakkında",
        about_developer: "koder-cog",
        about_comments: "Turnike, GSB ve KYK yurt ağlarında captive portal üzerinden oturum açmayı, askıda kalan bağlantıları devralmayı ve kalan kota bilgilerini anlık takip etmeyi sağlayan modern masaüstü uygulamasıdır.",
        about_source_code: "Kaynak Kodu",
    },

    status: StatusTexts {
        checking_connection: "Bağlantı denetleniyor",
        connected: "Bağlı",
        not_logged_in: "Oturum kapalı",
        no_connection: "Ağ bağlantısı yok",
        login_failed: "Oturum açılamadı",
        foreign_network: "GSBWIFI’a bağlı değil",
    },
    network: NetworkTexts {
        foreign_wifi_toast: "GSBWIFI ağına bağlanın",
        disconnected_banner: "Ağ bağlantısı yok",
    },
    auth: AuthTexts {
        username_label: "Kimlik veya pasaport numarası",
        password_label: "Şifre",
        remember_me: "Kimlik bilgilerini hatırla",
        login_btn: "Oturum Aç",
        logout_btn: "Oturumu Kapat",
        logout_dialog_heading: "Oturumu Kapat",
        logout_dialog_body: "Yurt ağı oturumu kapatılacak.",
        logout_dialog_cancel: "Vazgeç",
        logout_dialog_confirm: "Oturumu Kapat",
        register_link: "Yeni Kullanıcı Kaydı…",
        forgot_password_link: "Şifre Sıfırlama…",
    },
    quota: QuotaTexts {
        title: "Kotalar",
        social: "Sosyal Medya",
        general: "Genel İnternet",
        session_time: "Ağ Süresi",
        wifi_ap: "Erişim Noktası",
    },
    projection: ProjectionTexts {
        title: "Kullanım Tahmini",
        daily_safe_budget: "Günlük Bütçe",
        current_burn_rate: "Günlük Harcama",
    },
    preferences: PreferencesTexts {
        title: "Tercihler",
        appearance_label: "Görünüm",
        appearance_options: ["Sistem", "Açık", "Koyu"],
        language_label: "Dil",
        service_group: "Arka Plan Servisi",
        autostart: RowText::new("Açılışta Başlat", ""),
        auto_login: RowText::new("Ağ Algılandığında Bağlan", ""),
        auto_preempt: RowText::new("Oturumu Devral", "Diğer cihazın oturumunu kapat"),
        auto_metered: RowText::new("Tarifeli Bağlantı", "Arka plan veri tüketimini kısıtla"),
        auto_logout_on_screen_lock: RowText::new("Ekran Kilitlendiğinde Oturumu Kapat", ""),
        radar: RowText::new("Erişim Noktası Radarı", "Daha güçlü sinyale otomatik geç"),
        quota_group: "Kota Bildirimleri",
        notify_quota: RowText::new("Düşük Kota Bildirimi", ""),
        threshold_mode_label: "Eşik Türü",
        threshold_mode_options: ["Yüzde (%)", "Veri (GB)"],
        threshold_value_label: "Uyarı Eşiği",
    },
    password: PasswordTexts {
        change_title: "Şifre Değiştir",
        current_label: "Mevcut şifre",
        temporary_label: "e-Devlet geçici şifresi",
        first_label: "İlk şifre",
        new_label: "Yeni şifre",
        confirm_label: "Yeni şifre (tekrar)",
        rules_title: "Kurallar",
        rule_length: "6 ile 10 karakter uzunluğunda olmalı",
        rule_chars: "Büyük harf, küçük harf ve rakam içermeli",
        rule_forbidden_chars: "Şu karakterleri içermemeli: '|{}()[]",
        rule_forbidden_letters: "Türkçe harf ve £ içermemeli",
        rule_no_username: "Kimlik numarası içeremez",
        save_btn: "Şifreyi Güncelle",
        updated_toast: "Şifre güncellendi",
        mismatch_toast: "Şifreler eşleşmiyor",
    },
    register: RegisterTexts {
        title: "Yeni Kullanıcı Kaydı",
        nationality_label: "Uyruk",
        nationality_tc: "T.C. vatandaşı",
        nationality_foreigner: "Yabancı uyruklu",
        user_type_label: "Kullanıcı tipi",
        user_types: [
            "Öğrenci",
            "İdari personel",
            "Gençlik merkezi kullanıcısı",
            "Gençlik merkezi personeli",
            "TOHM kullanıcısı",
            "SEM kullanıcısı",
        ],
        verify_btn: "Devam Et",
        edevlet_info: "e-Devlet’teki GSB Wi-Fi Şifre İşlemleri sayfasından aldığınız geçici şifreyi 15 dakika içinde kullanabilirsiniz",
    },
    actions: ActionsTexts {
        title: "Son Hareketler",
        loading: "Son Hareketler Yükleniyor",
        refresh_tooltip: "Yenile",
        no_records: "Kayıt Bulunamadı",
        summary_title: "Kullanım Özeti",
        download: "İndirme",
        upload: "Yükleme",
    },
    help: HelpTexts {
        title: "Yardım",
        sections: [
            (
                "Hesap ve İlk Giriş",
                "Kullanıcı adınız T.C. kimlik veya pasaport numaranızdır. Yurt idaresinden aldığınız ilk şifreyle Yeni Kullanıcı Kaydı ekranından hesabınızı açıp kendi şifrenizi belirleyebilirsiniz.",
            ),
            (
                "Şifre Sıfırlama",
                "Şifrenizi unuttuysanız e-Devlet’te GSB Wi-Fi Şifre İşlemleri sayfasından geçici şifre alıp 15 dakika içinde Şifre Sıfırlama ekranından yeni şifrenizi oluşturabilirsiniz.",
            ),
            (
                "Oturum Süresi ve Cihaz Sınırı",
                "Oturumu kapatmadığınızda oturumunuz 8 saat boyunca açık kalır. Başka bir cihazdan bağlanırken maksimum oturum uyarısı almamak için işiniz bittiğinde oturumu kapatın.",
            ),
            (
                "Kotasız Üniversite Erişimi",
                "Oturum kapalıyken üniversite sitelerine (.edu.tr) ve ULAKBİM akademik kaynaklarına kotadan harcanmadan doğrudan girilebilir.",
            ),
        ],
        shortcuts_list: [
            ("Durumu yenile", "<Control>r"),
            ("Tercihler", "<Control>comma"),
            ("Yardım", "F1"),
            ("Sayfayı kapat", "<Control>w"),
            ("Oturum aç", "Return"),
            ("Klavye kısayolları", "<Control>question"),
            ("Uygulamadan çık", "<Control>q"),
        ],
    },
};

pub static EN_CATALOG: Catalog = Catalog {
    app: AppTexts {
        title: "Turnike",
        main_menu_tooltip: "Main Menu",
        preferences_menu: "Preferences",
        shortcuts: "Keyboard Shortcuts",
        about: "About Turnike",
        about_developer: "koder-cog",
        about_comments: "Turnike is an open-source desktop application designed to manage captive portal sessions, preempt dangling connections, and track data quotas on GSB and KYK dormitory networks.",
        about_source_code: "Source Code",
    },

    status: StatusTexts {
        checking_connection: "Checking Connection",
        connected: "Connected",
        not_logged_in: "Signed Out",
        no_connection: "No Network Connection",
        login_failed: "Sign-In Failed",
        foreign_network: "Not Connected to GSBWIFI",
    },
    network: NetworkTexts {
        foreign_wifi_toast: "Connect to GSBWIFI",
        disconnected_banner: "No network connection",
    },
    auth: AuthTexts {
        username_label: "ID or passport number",
        password_label: "Password",
        remember_me: "Remember Credentials",
        login_btn: "Sign In",
        logout_btn: "Sign Out",
        logout_dialog_heading: "Log Out",
        logout_dialog_body: "The network session will be closed.",
        logout_dialog_cancel: "Cancel",
        logout_dialog_confirm: "Log Out",
        register_link: "New User Registration…",
        forgot_password_link: "Reset Password…",
    },
    quota: QuotaTexts {
        title: "Quotas",
        social: "Social Media",
        general: "General Internet",
        session_time: "Network Duration",
        wifi_ap: "Access Point",
    },
    projection: ProjectionTexts {
        title: "Usage Estimate",
        daily_safe_budget: "Daily Budget",
        current_burn_rate: "Daily Usage",
    },
    preferences: PreferencesTexts {
        title: "Preferences",
        appearance_label: "Appearance",
        appearance_options: ["System", "Light", "Dark"],
        language_label: "Language",
        service_group: "Background Service",
        autostart: RowText::new("Start on Boot", ""),
        auto_login: RowText::new("Connect on Network Detection", ""),
        auto_preempt: RowText::new("Take Over Session", "Sign out on other devices"),
        auto_metered: RowText::new("Metered Connection", "Limit background data usage"),
        auto_logout_on_screen_lock: RowText::new("Sign Out on Screen Lock", ""),
        radar: RowText::new(
            "Access Point Radar",
            "Switch to stronger signal automatically",
        ),
        quota_group: "Quota Notifications",
        notify_quota: RowText::new("Low Quota Alert", ""),
        threshold_mode_label: "Threshold type",
        threshold_mode_options: ["Percent (%)", "Data (GB)"],
        threshold_value_label: "Alert threshold",
    },
    password: PasswordTexts {
        change_title: "Change Password",
        current_label: "Current password",
        temporary_label: "e-Government temporary password",
        first_label: "Initial password",
        new_label: "New password",
        confirm_label: "Confirm new password",
        rules_title: "Requirements",
        rule_length: "6 to 10 characters long",
        rule_chars: "At least one uppercase letter, lowercase letter, and digit",
        rule_forbidden_chars: "Must not contain: '|{}()[]",
        rule_forbidden_letters: "Must not contain Turkish letters or £ symbol",
        rule_no_username: "Must not contain username or ID number",
        save_btn: "Update Password",
        updated_toast: "Password updated",
        mismatch_toast: "Passwords do not match",
    },
    register: RegisterTexts {
        title: "New User Registration",
        nationality_label: "Nationality",
        nationality_tc: "Turkish citizen",
        nationality_foreigner: "Foreign national",
        user_type_label: "User type",
        user_types: [
            "Student",
            "Administrative staff",
            "Youth center user",
            "Youth center staff",
            "TOHM user",
            "SEM user",
        ],
        verify_btn: "Continue",
        edevlet_info: "Use the temporary password from e-Government GSB Wi-Fi Password Services within 15 minutes",
    },
    actions: ActionsTexts {
        title: "Recent Activity",
        loading: "Loading Recent Activity",
        refresh_tooltip: "Refresh",
        no_records: "No Records Found",
        summary_title: "Usage Summary",
        download: "Download",
        upload: "Upload",
    },
    help: HelpTexts {
        title: "Help",
        sections: [
            (
                "Account and First Sign-In",
                "Your username is your national ID or passport number. Use the initial password from your dormitory office in New User Registration to activate your account and set a personal password.",
            ),
            (
                "Resetting Your Password",
                "Generate a temporary password on e-Government under GSB Wi-Fi Password Services and use it within 15 minutes in Reset Password.",
            ),
            (
                "Session Duration and Device Limit",
                "Sessions stay active for up to 8 hours unless you sign out. End your session before switching devices to avoid hitting the active session limit.",
            ),
            (
                "Quota-Free University Access",
                "While signed out, you can browse university websites (.edu.tr) and ULAKBİM academic resources without using your monthly quota.",
            ),
        ],
        shortcuts_list: [
            ("Refresh status", "<Control>r"),
            ("Preferences", "<Control>comma"),
            ("Help", "F1"),
            ("Close page", "<Control>w"),
            ("Sign in", "Return"),
            ("Keyboard shortcuts", "<Control>question"),
            ("Quit application", "<Control>q"),
        ],
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_integrity() {
        for lang in [Language::Tr, Language::En] {
            let t = lang.texts();
            assert!(!t.app.title.is_empty());
            assert!(!t.preferences.title.is_empty());
            assert!(!t.preferences.autostart.title.is_empty());
            assert!(!t.preferences.auto_login.title.is_empty());
            assert!(!t.preferences.auto_preempt.title.is_empty());
            assert!(!t.preferences.auto_metered.title.is_empty());
            assert!(!t.preferences.quota_group.is_empty());
            assert!(!t.preferences.notify_quota.title.is_empty());
            assert!(!t.preferences.threshold_mode_label.is_empty());
            assert_eq!(t.preferences.threshold_mode_options.len(), 2);
            assert!(!t.preferences.threshold_value_label.is_empty());
            assert!(!t.projection.title.is_empty());
            assert!(!t.projection.daily_safe_budget.is_empty());
            assert!(!t.projection.current_burn_rate.is_empty());
            assert_eq!(t.help.sections.len(), 4);
            assert_eq!(t.help.shortcuts_list.len(), 7);
        }
    }

    #[test]
    fn test_quota_countdown_formatting() {
        let tr = Language::Tr.format_quota_reset_countdown("2 4 15");
        assert_eq!(tr, "2 gün 4 sa 15 dk");

        let en = Language::En.format_quota_reset_countdown("2 4 15");
        assert_eq!(en, "2d 4h 15m");

        assert_eq!(
            Language::Tr.format_quota_reset_countdown("2 0 15"),
            "2 gün 15 dk"
        );
        assert_eq!(
            Language::En.format_quota_reset_countdown("2 0 15"),
            "2d 15m"
        );

        assert_eq!(
            Language::Tr.format_quota_reset_countdown("0 4 15"),
            "4 sa 15 dk"
        );
        assert_eq!(
            Language::En.format_quota_reset_countdown("0 4 15"),
            "4h 15m"
        );

        assert_eq!(Language::Tr.format_quota_reset_countdown("0 0 15"), "15 dk");
        assert_eq!(Language::En.format_quota_reset_countdown("0 0 15"), "15m");

        assert_eq!(Language::Tr.format_quota_reset_countdown("0 0 0"), "0 dk");
        assert_eq!(Language::En.format_quota_reset_countdown("0 0 0"), "0m");
    }

    #[test]
    fn test_duration_formatting() {
        let tr = Language::Tr.format_portal_duration("0 3 20 10");
        assert_eq!(tr, "3 sa 20 dk 10 sn");

        let en = Language::En.format_portal_duration("0 3 20 10");
        assert_eq!(en, "3h 20m 10s");
    }

    #[test]
    fn test_duration_seconds_formatting() {
        assert_eq!(Language::Tr.format_duration_seconds(0), "0 sn");
        assert_eq!(Language::Tr.format_duration_seconds(45), "45 sn");
        assert_eq!(Language::Tr.format_duration_seconds(75), "1 dk 15 sn");
        assert_eq!(Language::Tr.format_duration_seconds(3665), "1 sa 1 dk 5 sn");
        assert_eq!(Language::En.format_duration_seconds(3665), "1h 1m 5s");
        assert_eq!(Language::Tr.format_duration_seconds(90000), "1 gün 1 sa");
        assert_eq!(Language::En.format_duration_seconds(90000), "1d 1h");
        assert_eq!(
            Language::Tr.format_duration_seconds(90060),
            "1 gün 1 sa 1 dk"
        );
        assert_eq!(Language::En.format_duration_seconds(90060), "1d 1h 1m");
    }
}
