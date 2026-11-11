use regex::Regex;
use std::sync::LazyLock;
use turnike_types::{FaqItem, LastActionItem, PortalError, QuotaInfo, QuotaPackage};

// PrimeFaces 6.0 dinamik bileşen ID'leri (j_idt127, j_idt135 vb.) ve CDATA blokları üretir.
// Ağır bir DOM ayrıştırıcısı (html5ever) yerine derleme zamanı ve binary boyutunu hafif tutmak
// ve dinamik buton/ViewState alanlarını doğrudan yakalamak için önceden derlenmiş LazyLock regex'ler kullanılır.
static RE_VIEW_STATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<input[^>]*?(?:name="javax\.faces\.ViewState"[^>]*?value="([^"]+)"|value="([^"]+)"[^>]*?name="javax\.faces\.ViewState")"#)
        .expect("ViewState regex deseni geçersiz")
});

static RE_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<span class="myinfo">\s*([^<]+?)\s*</span>"#)
        .expect("Kullanıcı adı regex deseni geçersiz")
});

static RE_SESSION_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:Session Time|Oturum Süresi):[\s\S]*?<label[^>]*?>\s*([^<]+?)\s*</label>"#)
        .expect("Oturum süresi regex deseni geçersiz")
});

static RE_LOGIN_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:Login Time|Login Zamanı|Giriş Zamanı):[\s\S]*?<label[^>]*?>\s*([^<]+?)\s*</label>"#,
    )
    .expect("Giriş zamanı regex deseni geçersiz")
});

static RE_TABLE_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<table[^>]*?class="[^"]*?panelGrid[^"]*?".*?</table>"#)
        .expect("PanelGrid tablosu regex deseni geçersiz")
});

static RE_PACKAGE_TYPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:Package Type|Paket Tipi):[\s\S]*?<span[^>]*?>\s*([^<]+?)\s*</span>"#)
        .expect("Paket tipi regex deseni geçersiz")
});

static RE_TOTAL_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:Total Quota \(MB\)|Toplam Kota \(MB\)):[\s\S]*?<label[^>]*?>\s*([\d\.]+)\s*</label>"#,
    )
    .expect("Toplam kota regex deseni geçersiz")
});

static RE_REMAINING_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:Total Remaining Quota \(MB\)|Toplam Kalan Kota \(MB\)):[\s\S]*?<label[^>]*?>\s*([\d\.]+)\s*</label>"#,
    )
    .expect("Kalan kota regex deseni geçersiz")
});

static RE_START_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:Start Date|Başlangıç Tarihi):[\s\S]*?<label[^>]*?>\s*([^<]+?)\s*</label>"#)
        .expect("Başlangıç tarihi regex deseni geçersiz")
});

static RE_EXPIRATION_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:Expiration Date|Sona Erme Tarihi|Bitiş Tarihi):[\s\S]*?<label[^>]*?>\s*([^<]+?)\s*</label>"#,
    )
    .expect("Bitiş tarihi regex deseni geçersiz")
});

static RE_REMAINING_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:Remaining Quota Time|Kalan Kota Zamanı|Kalan Kota Süresi):[\s\S]*?<label[^>]*?>\s*([^<]+?)\s*</label>"#,
    )
    .expect("Kalan süre regex deseni geçersiz")
});

static RE_LOGOUT_BUTTON_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"<button[^>]*?id="([^"]+)"[^>]*?>\s*<span[^>]*?>\s*(?:End Session|Oturumu Sonlandır|Oturumu Kapat)\s*</span>"#,
    )
    .expect("Çıkış butonu regex deseni geçersiz")
});

static RE_ACTION_ROW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<tr[^>]*?role="row"[^>]*?>(.*?)</tr>"#)
        .expect("Oturum hareket satırı regex deseni geçersiz")
});

static RE_ACTION_CELL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<td[^>]*?role="gridcell"[^>]*?>(.*?)</td>"#)
        .expect("Oturum hareket hücresi regex deseni geçersiz")
});

static RE_TAG_STRIP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<[^>]+>"#).expect("HTML tag temizleyici regex deseni geçersiz"));

static RE_UI_MESSAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div[^>]*?class="[^"]*?ui-messages[^"]*?".*?<span[^>]*?class="[^"]*?ui-messages-(?:error|info|warn|fatal)-(?:summary|detail)[^"]*?"[^>]*?>\s*([^<]+?)\s*</span>"#)
        .expect("UI mesaj regex deseni geçersiz")
});

static RE_UI_ERROR_BOX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div[^>]*?class="[^"]*?(?:ui-messages-(?:error|fatal)|ui-message-(?:error|fatal))[^"]*?"[^>]*?>(.*?)</div>"#)
        .expect("UI hata kutusu regex deseni geçersiz")
});

static RE_UI_INFO_BOX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div[^>]*?class="[^"]*?(?:ui-messages-info|ui-message-info)[^"]*?"[^>]*?>(.*?)</div>"#)
        .expect("UI bilgi kutusu regex deseni geçersiz")
});

static RE_SUMMARY_SPAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<span[^>]*?class="[^"]*?ui-messages?-(?:error|fatal|warn|info)-summary[^"]*?"[^>]*?>\s*([^<]+?)\s*</span>"#)
        .expect("UI summary span regex deseni geçersiz")
});

static RE_DETAIL_SPAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<span[^>]*?class="[^"]*?ui-messages?-(?:error|fatal|warn|info)-detail[^"]*?"[^>]*?>\s*([^<]+?)\s*</span>"#)
        .expect("UI detail span regex deseni geçersiz")
});

static RE_XML_ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<error-message>(?:<!\[CDATA\[)?(.*?)(?:\]\]>)?</error-message>"#)
        .expect("XML error message regex deseni geçersiz")
});

static RE_GROWL_MESSAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"msgs:\[\{[^\]]*?(?:detail|summary)\s*:\s*"([^"]+)""#)
        .expect("Growl mesaj regex deseni geçersiz")
});

static RE_LOGIN_ERROR_MYINFO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<span[^>]*?class="[^"]*?myinfo[^"]*?"[^>]*?style="[^"]*?background-color:\s*red[^"]*?"[^>]*?>\s*([^<]+?)\s*</span>"#)
        .expect("Login hata mesajı regex deseni geçersiz")
});

static RE_AJAX_REDIRECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<redirect[^>]*?url="([^"]+)""#)
        .expect("PrimeFaces AJAX redirect regex deseni geçersiz")
});

static RE_FAQ_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<p[^>]*?class="[^"]*?myQuestion[^"]*?"[^>]*?>(.*?)</p>\s*<p[^>]*?class="[^"]*?myAnswer[^"]*?"[^>]*?>(.*?)</p>"#,
    )
    .expect("SSS soru-cevap regex deseni geçersiz")
});

static RE_FORM_SUBMIT_BUTTON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<button[^>]*?id="([^"]+)"[^>]*?type="submit"[^>]*?>(.*?)</button>"#)
        .expect("Submit butonu regex deseni geçersiz")
});

static RE_ATTR_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id="([^"]+)""#).expect("Id attribute regex deseni geçersiz"));

static RE_ATTR_ACTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"action="([^"]*)""#).expect("Action attribute regex deseni geçersiz")
});

static RE_CHECKBOX_INPUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<input[^>]*?type="checkbox"[^>]*?>"#)
        .expect("Checkbox input regex deseni geçersiz")
});

static RE_RADIO_INPUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<input[^>]*?type="radio"[^>]*?>"#).expect("Radio input regex deseni geçersiz")
});

static RE_ATTR_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"name="([^"]+)""#).expect("Name attribute regex deseni geçersiz")
});

static RE_ATTR_VALUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"value="([^"]*)""#).expect("Value attribute regex deseni geçersiz")
});

static RE_TEXT_INPUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<input[^>]*?type="text"[^>]*?>|<textarea[^>]*?>"#)
        .expect("Text input regex deseni geçersiz")
});

static RE_FORM_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<form\b([^>]*)>(.*?)</form>"#).expect("Form tag regex deseni geçersiz")
});

pub fn extract_view_state(html: &str) -> Result<String, PortalError> {
    RE_VIEW_STATE
        .captures(html)
        .and_then(|cap| cap.get(1).or_else(|| cap.get(2)))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| PortalError::ParseError("javax.faces.ViewState bulunamadı".to_string()))
}

pub fn extract_logout_button_id(html: &str) -> Option<String> {
    RE_LOGOUT_BUTTON_ID
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn extract_form_submit_button_id(html: &str, form_prefix: &str) -> Option<String> {
    for cap in RE_FORM_SUBMIT_BUTTON.captures_iter(html) {
        if let (Some(id_match), Some(content_match)) = (cap.get(1), cap.get(2)) {
            let id = id_match.as_str();
            let content = content_match.as_str().to_lowercase();
            if id.starts_with(form_prefix)
                && !content.contains("back")
                && !content.contains("geri")
                && !content.contains("main page")
                && !content.contains("ana sayfa")
            {
                return Some(id.to_string());
            }
        }
    }
    None
}

pub fn extract_register_button_id(html: &str) -> Option<String> {
    extract_form_submit_button_id(html, "takeFirstInfoForm")
}

pub fn extract_forgot_password_button_id(html: &str) -> Option<String> {
    extract_form_submit_button_id(html, "passwordForgetForm")
}

pub fn extract_change_password_button_id(html: &str) -> Option<String> {
    extract_form_submit_button_id(html, "passwordUpdateForm")
}

pub fn parse_quota_page(html: &str) -> Result<QuotaInfo, PortalError> {
    if let Some(err) = extract_portal_error(html) {
        return Err(PortalError::AuthenticationFailed(err));
    }

    if html.contains("j_spring_security_check")
        || html.contains("takeFirstInfoForm")
        || html.contains("passwordForgetForm")
        || html.contains("login.html")
    {
        return Err(PortalError::ParseError(
            "Sayfa oturum açma formu içeriyor, aktif oturum yok".to_string(),
        ));
    }

    let full_name = RE_NAME
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string());

    let session_time = RE_SESSION_TIME
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string());

    let login_time = RE_LOGIN_TIME
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string());

    let mut packages = Vec::new();

    for table_match in RE_TABLE_BLOCK.find_iter(html) {
        let table_html = table_match.as_str();

        let package_type = match RE_PACKAGE_TYPE.captures(table_html).and_then(|c| c.get(1)) {
            Some(m) => m.as_str().trim().to_string(),
            None => continue,
        };

        let total_mb: f64 = RE_TOTAL_QUOTA
            .captures(table_html)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0.0);

        let remaining_mb: f64 = RE_REMAINING_QUOTA
            .captures(table_html)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0.0);

        let start_date = RE_START_DATE
            .captures(table_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        let expiration_date = RE_EXPIRATION_DATE
            .captures(table_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        let remaining_time = RE_REMAINING_TIME
            .captures(table_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        packages.push(QuotaPackage {
            package_type,
            total_mb,
            remaining_mb,
            start_date,
            expiration_date,
            remaining_time,
        });
    }

    let has_session_marker = html.contains("servisUpdateForm")
        || html.contains("kullaniciDetayBilgileriForm")
        || extract_logout_button_id(html).is_some()
        || session_time.is_some()
        || login_time.is_some();

    if packages.is_empty() && !has_session_marker {
        return Err(PortalError::ParseError(
            "Sayfa içeriğinde geçerli kota veya kullanıcı verisi bulunamadı".to_string(),
        ));
    }

    Ok(QuotaInfo {
        packages,
        session_time,
        login_time,
        full_name,
    })
}

pub fn parse_last_actions(html: &str) -> Result<Vec<LastActionItem>, PortalError> {
    if let Some(err) = extract_portal_error(html) {
        return Err(PortalError::AuthenticationFailed(err));
    }

    if html.contains("j_spring_security_check")
        || html.contains("login.html")
        || html.contains("takeFirstInfoForm")
        || html.contains("passwordForgetForm")
    {
        return Err(PortalError::AuthenticationFailed(
            "Oturum süresi dolmuş veya giriş yapılmamış".to_string(),
        ));
    }

    let mut actions = Vec::new();

    for row_match in RE_ACTION_ROW.find_iter(html) {
        let row_html = row_match.as_str();
        let mut cells = Vec::new();

        for cell_match in RE_ACTION_CELL.find_iter(row_html) {
            let raw_text = cell_match.as_str();
            let clean_text = RE_TAG_STRIP.replace_all(raw_text, "").trim().to_string();
            cells.push(clean_text);
        }

        if cells.len() >= 5 {
            let check_in_time = cells[0].clone();
            let dormitory_name = cells[1].clone();
            let download_mb: f64 = cells[2].parse().unwrap_or(0.0);
            let upload_mb: f64 = cells[3].parse().unwrap_or(0.0);
            let duration_seconds: u64 = cells[4].parse().unwrap_or(0);
            let service = cells.get(5).cloned().unwrap_or_default();

            actions.push(LastActionItem {
                check_in_time,
                dormitory_name,
                download_mb,
                upload_mb,
                duration_seconds,
                service,
            });
        }
    }

    let is_actions_page = html.contains("sonHareketler")
        || html.contains("tblSonHareketler")
        || html.contains("servisUpdateForm")
        || html.contains("panelGrid")
        || html.contains("myinfo");

    if actions.is_empty() && !is_actions_page {
        return Err(PortalError::ParseError(
            "Son hareketler tablosu bulunamadı".to_string(),
        ));
    }

    Ok(actions)
}

pub fn extract_portal_error(html: &str) -> Option<String> {
    if let Some(msg) = RE_LOGIN_ERROR_MYINFO
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return Some(msg);
    }

    if let Some(msg) = RE_XML_ERROR
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return Some(msg);
    }

    if let Some(cap) = RE_UI_ERROR_BOX.captures(html) {
        let inner = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let summary = RE_SUMMARY_SPAN
            .captures(inner)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());
        let detail = RE_DETAIL_SPAN
            .captures(inner)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());

        match (summary, detail) {
            (Some(s), Some(d)) => {
                let s_clean = s.trim_end_matches(':').trim();
                let d_clean = d.trim();
                if s_clean.eq_ignore_ascii_case("hata")
                    || s_clean.eq_ignore_ascii_case("error")
                    || s_clean == d_clean
                {
                    return Some(d_clean.to_string());
                }
                return Some(format!("{s_clean}: {d_clean}"));
            }
            (Some(s), None) => return Some(s),
            (None, Some(d)) => return Some(d),
            (None, None) => {
                let cleaned = RE_TAG_STRIP.replace_all(inner, "").trim().to_string();
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
    }

    if html.contains("ui-messages-error")
        || html.contains("ui-message-error")
        || html.contains("ui-messages-fatal")
    {
        if let Some(msg) = RE_DETAIL_SPAN
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(msg);
        }
        if let Some(msg) = RE_SUMMARY_SPAN
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(msg);
        }
    }

    if (html.contains(r#""severity":"error""#) || html.contains(r#""severity":"fatal""#))
        && let Some(cap) = RE_GROWL_MESSAGE.captures(html)
        && let Some(m) = cap.get(1)
    {
        let s = m.as_str().trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }

    None
}

pub fn extract_portal_success(html: &str) -> Option<String> {
    if let Some(cap) = RE_UI_INFO_BOX.captures(html) {
        let inner = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let summary = RE_SUMMARY_SPAN
            .captures(inner)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());
        let detail = RE_DETAIL_SPAN
            .captures(inner)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());

        match (summary, detail) {
            (Some(s), Some(d)) => {
                let s_clean = s.trim_end_matches(':').trim();
                let d_clean = d.trim();
                if s_clean.eq_ignore_ascii_case("bilgi")
                    || s_clean.eq_ignore_ascii_case("info")
                    || s_clean == d_clean
                {
                    return Some(d_clean.to_string());
                }
                return Some(format!("{s_clean}: {d_clean}"));
            }
            (Some(s), None) => return Some(s),
            (None, Some(d)) => return Some(d),
            (None, None) => {
                let cleaned = RE_TAG_STRIP.replace_all(inner, "").trim().to_string();
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
    }

    if html.contains(r#""severity":"info""#)
        && let Some(cap) = RE_GROWL_MESSAGE.captures(html)
        && let Some(m) = cap.get(1)
    {
        let s = m.as_str().trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }

    None
}

pub fn extract_ui_message(html: &str) -> Option<String> {
    extract_portal_error(html)
        .or_else(|| extract_portal_success(html))
        .or_else(|| {
            RE_UI_MESSAGE
                .captures(html)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            RE_GROWL_MESSAGE
                .captures(html)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

pub fn extract_ajax_redirect(xml: &str) -> Option<String> {
    RE_AJAX_REDIRECT
        .captures(xml)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn is_max_sessions_message(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("maksimum")
        || lower.contains("maximum")
        || lower.contains("oturum sayısı")
        || lower.contains("oturum sayisi")
        || lower.contains("başka bir cihaz")
        || lower.contains("baska bir cihaz")
        || lower.contains("tek bir cihaz")
        || lower.contains("aktif bir oturum")
        || lower.contains("mevcut bir oturum")
}

pub fn parse_faq_page(html: &str) -> Vec<FaqItem> {
    let mut items = Vec::new();
    for cap in RE_FAQ_ITEM.captures_iter(html) {
        if let (Some(q), Some(a)) = (cap.get(1), cap.get(2)) {
            let question = RE_TAG_STRIP.replace_all(q.as_str(), "").trim().to_string();
            let answer = RE_TAG_STRIP.replace_all(a.as_str(), "").trim().to_string();
            if !question.is_empty() && !answer.is_empty() {
                items.push(FaqItem { question, answer });
            }
        }
    }
    items
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurveyForm {
    pub form_id: String,
    pub action: String,
    pub view_state: String,
    pub submit_button_id: String,
    pub radio_groups: Vec<(String, Vec<String>)>,
    pub text_inputs: Vec<String>,
    pub checkboxes: Vec<String>,
}

pub fn is_survey_page(html: &str) -> bool {
    // Oturum henüz açılmamışsa veya form giriş şifresi bekliyorsa anket sayılmaz
    if html.contains("j_spring_security_check") && html.contains("j_password") {
        return false;
    }

    let lower = html.to_lowercase();

    // Portal zorunlu anket yönlendirmesi yaptığında surveyRequired.html sayfası döner
    if html.contains("surveyRequired")
        || html.contains("surveyForm")
        || lower.contains("anketi cevapladıktan sonra")
        || lower.contains("zorunlu anket")
        || lower.contains("mandatory survey")
        || lower.contains("surveywar")
    {
        return true;
    }

    // Portal akordeon panelinde anket sekmesi varsa ve boş anket uyarısı içermiyorsa
    if (html.contains("mainPanel:surveyTab") || html.contains("surveyTab"))
        && !html.contains("mainPanel:no_survey_main:no_survey")
        && !lower.contains("there are no surveys")
        && !lower.contains("anket bulunmamaktadır")
    {
        return true;
    }

    // Çıkış butonu yokken sayfada soru formuna işaret eden radio butonları bulunuyorsa
    if html.contains(r#"type="radio""#)
        && (html.contains("survey") || html.contains("anket") || !html.contains("servisUpdateForm"))
    {
        return true;
    }

    false
}

pub fn extract_survey_form(html: &str) -> Result<SurveyForm, PortalError> {
    let view_state = extract_view_state(html)?;

    // PrimeFaces anket formunu bulmak için önce formu kapsayan tag aranır
    let mut target_form_id = None;
    let mut target_action = None;
    let mut target_form_html = html;

    for cap in RE_FORM_TAG.captures_iter(html) {
        if let (Some(attrs_m), Some(content_m)) = (cap.get(1), cap.get(2)) {
            let attrs = attrs_m.as_str();
            let content = content_m.as_str();
            let id = RE_ATTR_ID
                .captures(attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
            let action = RE_ATTR_ACTION
                .captures(attrs)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());

            let is_survey_target = if let Some(ref form_id) = id {
                let id_lower = form_id.to_lowercase();
                id_lower.contains("survey")
                    || id_lower.contains("anket")
                    || content.contains(r#"type="radio""#)
            } else {
                content.contains(r#"type="radio""#)
            };

            if is_survey_target {
                target_form_id = id;
                target_action = action;
                target_form_html = content;
                break;
            }
        }
    }

    let form_id = target_form_id.unwrap_or_else(|| "mainPanel:surveyTab".to_string());
    let action = target_action.unwrap_or_else(|| "/surveyRequired.html".to_string());

    let mut radio_groups: Vec<(String, Vec<String>)> = Vec::new();
    for cap in RE_RADIO_INPUT.find_iter(target_form_html) {
        let tag = cap.as_str();
        let name = RE_ATTR_NAME
            .captures(tag)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());
        let value = RE_ATTR_VALUE
            .captures(tag)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        if let Some(n) = name {
            if let Some(existing) = radio_groups.iter_mut().find(|(k, _)| k == &n) {
                if !existing.1.contains(&value) {
                    existing.1.push(value);
                }
            } else {
                radio_groups.push((n, vec![value]));
            }
        }
    }

    let mut text_inputs = Vec::new();
    for cap in RE_TEXT_INPUT.find_iter(target_form_html) {
        let tag = cap.as_str();
        if let Some(name_m) = RE_ATTR_NAME.captures(tag).and_then(|c| c.get(1)) {
            let n = name_m.as_str();
            let n_lower = n.to_lowercase();
            if !n_lower.contains("viewstate")
                && !n_lower.contains("activeindex")
                && !n_lower.contains("scrollstate")
                && !n_lower.contains("locale")
                && !text_inputs.contains(&n.to_string())
            {
                text_inputs.push(n.to_string());
            }
        }
    }

    let mut checkboxes = Vec::new();
    for cap in RE_CHECKBOX_INPUT.find_iter(target_form_html) {
        let tag = cap.as_str();
        if let Some(name_m) = RE_ATTR_NAME.captures(tag).and_then(|c| c.get(1)) {
            let n = name_m.as_str();
            if !checkboxes.contains(&n.to_string()) {
                checkboxes.push(n.to_string());
            }
        }
    }

    // Buton seçim hiyerarşisi:
    // 1. "Daha Sonra Hatırlat" (remind) butonu oturumu doğrudan onaylatıp interneti açar.
    // 2. "Başlat" (start) butonu bazı portal varyasyonlarında onay sonrası interneti başlatır.
    // 3. "Kaydet / Gönder" (submit) butonu anketin yanıtlarıyla birlikte kaydedilmesini sağlar.
    let mut remind_button_id = None;
    let mut start_button_id = None;
    let mut submit_button_id = None;
    let mut first_button_id = None;

    for cap in RE_FORM_SUBMIT_BUTTON.captures_iter(target_form_html) {
        if let (Some(id_m), Some(text_m)) = (cap.get(1), cap.get(2)) {
            let id = id_m.as_str();
            let text = text_m.as_str().to_lowercase();
            let id_lower = id.to_lowercase();

            if first_button_id.is_none() {
                first_button_id = Some(id.to_string());
            }

            if text.contains("daha sonra")
                || text.contains("hatırlat")
                || text.contains("remind")
                || text.contains("atla")
                || text.contains("skip")
                || id_lower.contains("remind")
            {
                remind_button_id = Some(id.to_string());
                break;
            } else if text.contains("başlat")
                || text.contains("start")
                || id_lower.contains("start")
            {
                start_button_id = Some(id.to_string());
            } else if text.contains("gönder")
                || text.contains("kaydet")
                || text.contains("tamamla")
                || text.contains("submit")
                || text.contains("save")
                || id_lower.contains("survey")
                || id_lower.contains("anket")
            {
                submit_button_id = Some(id.to_string());
            }
        }
    }

    let final_button = remind_button_id
        .or(start_button_id)
        .or(submit_button_id)
        .or(first_button_id)
        .unwrap_or_else(|| format!("{form_id}:submit"));

    Ok(SurveyForm {
        form_id,
        action,
        view_state,
        submit_button_id: final_button,
        radio_groups,
        text_inputs,
        checkboxes,
    })
}

// "I don't care about cookies" mantığıyla formdaki tüm sorular için rastgele
// makul seçenekler üretilir. Dış kütüphane bağımlılığı yerine nano zaman tohumu kullanılır.
pub fn build_random_survey_params(form: &SurveyForm) -> Vec<(String, String)> {
    let mut params = vec![
        ("javax.faces.partial.ajax".to_string(), "true".to_string()),
        (
            "javax.faces.source".to_string(),
            form.submit_button_id.clone(),
        ),
        (
            "javax.faces.partial.execute".to_string(),
            "@all".to_string(),
        ),
        (form.form_id.clone(), form.form_id.clone()),
        (form.submit_button_id.clone(), form.submit_button_id.clone()),
        ("javax.faces.ViewState".to_string(), form.view_state.clone()),
    ];

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(4242);

    for (idx, (radio_name, values)) in form.radio_groups.iter().enumerate() {
        if !values.is_empty() {
            let pick_idx = seed.wrapping_add(idx * 37) % values.len();
            params.push((radio_name.clone(), values[pick_idx].clone()));
        }
    }

    for cb in &form.checkboxes {
        params.push((cb.clone(), "on".to_string()));
    }

    const DUMMY_ANSWERS: [&str; 5] = ["İyi", "Teşekkürler", "Sorun yok", "Yeterli", "Normal"];
    for (idx, text_name) in form.text_inputs.iter().enumerate() {
        let pick_idx = seed.wrapping_add(idx * 17) % DUMMY_ANSWERS.len();
        params.push((text_name.clone(), DUMMY_ANSWERS[pick_idx].to_string()));
    }

    params
}

pub fn save_diagnostic_dump(html: &str, prefix: &str) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dump_dir = std::path::PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("turnike")
        .join("dumps");

    if let Err(e) = std::fs::create_dir_all(&dump_dir) {
        tracing::debug!("Teşhis döküm dizini oluşturulamadı: {e}");
        return;
    }

    let file_path = dump_dir.join(format!("{prefix}_{timestamp}.html"));
    if let Err(e) = std::fs::write(&file_path, html) {
        tracing::debug!("Teşhis dökümü yazılamadı: {e}");
    } else {
        tracing::info!("Portal teşhis dökümü kaydedildi: {}", file_path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_view_state() {
        let html = r#"<form><input type="hidden" name="javax.faces.ViewState" id="j_id1:javax.faces.ViewState:0" value="12345:67890" autocomplete="off" /></form>"#;
        assert_eq!(extract_view_state(html).unwrap(), "12345:67890");

        let html_reordered = r#"<form><input type="hidden" value="54321:09876" name="javax.faces.ViewState" autocomplete="off" /></form>"#;
        assert_eq!(extract_view_state(html_reordered).unwrap(), "54321:09876");
    }

    #[test]
    fn test_extract_logout_button_id() {
        let html = r#"<button id="servisUpdateForm:j_idt127" class="btn"><span class="ui-button-text">End Session</span></button>"#;
        assert_eq!(
            extract_logout_button_id(html).unwrap(),
            "servisUpdateForm:j_idt127"
        );
    }

    #[test]
    fn test_parse_last_actions() {
        let html = r#"
        <table role="grid">
          <tbody>
            <tr role="row">
              <td role="gridcell">14.10.2026 22:46:00</td>
              <td role="gridcell">ÖRNEK ÖĞRENCİ YURDU</td>
              <td role="gridcell">22.5</td>
              <td role="gridcell">3.2</td>
              <td role="gridcell">595</td>
              <td role="gridcell">WiFi</td>
            </tr>
          </tbody>
        </table>
        "#;
        let actions = parse_last_actions(html).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].check_in_time, "14.10.2026 22:46:00");
        assert_eq!(actions[0].dormitory_name, "ÖRNEK ÖĞRENCİ YURDU");
        assert_eq!(actions[0].download_mb, 22.5);
        assert_eq!(actions[0].upload_mb, 3.2);
        assert_eq!(actions[0].duration_seconds, 595);
        assert_eq!(actions[0].service, "WiFi");
    }

    #[test]
    fn test_extract_ui_message() {
        let html = r#"<div class="ui-messages"><span class="ui-messages-error-detail">Kullanıcı adı veya şifre hatalı.</span></div>"#;
        assert_eq!(
            extract_ui_message(html).as_deref(),
            Some("Kullanıcı adı veya şifre hatalı.")
        );
    }

    #[test]
    fn test_extract_portal_error_take_first_info() {
        let xml = r##"<partial-response id="j_id1"><changes><update id="j_idt18"><![CDATA[<div id="j_idt18" class="ui-outputpanel ui-widget">
<div id="takeFirstInfoForm:messages" class="ui-messages ui-widget" aria-live="polite"><div class="ui-messages-error ui-corner-all"><a href="#" class="ui-messages-close" onclick="$(this).parent().slideUp();return false;"><span class="ui-icon ui-icon-close"></span></a><span class="ui-messages-error-icon"></span><ul><li><span class="ui-messages-error-detail">Username used before</span></li></ul></div></div>
</div>]]></update></changes></partial-response>"##;
        assert_eq!(
            extract_portal_error(xml).as_deref(),
            Some("Username used before")
        );
    }

    #[test]
    fn test_extract_portal_error_forgot_password() {
        let xml = r##"<div id="passwordForgetForm:messagesPasswordForgetForm" class="ui-messages ui-widget" style="width:500px" aria-live="polite"><div class="ui-messages-error ui-corner-all"><a href="#" class="ui-messages-close" onclick="$(this).parent().slideUp();return false;"><span class="ui-icon ui-icon-close"></span></a><span class="ui-messages-error-icon"></span><ul><li><span class="ui-messages-error-summary">The provided information could not be verified. Please make sure your username and temporary password are correct and that the temporary password has not expired.</span></li></ul></div></div>"##;
        assert_eq!(
            extract_portal_error(xml).as_deref(),
            Some(
                "The provided information could not be verified. Please make sure your username and temporary password are correct and that the temporary password has not expired."
            )
        );
    }

    #[test]
    fn test_extract_portal_error_summary_and_detail() {
        let html = r#"<div class="ui-messages-error ui-corner-all"><span class="ui-messages-error-summary">Hata:</span><span class="ui-messages-error-detail">T.C. Kimlik No veya İlk Şifre Hatalı!</span></div>"#;
        assert_eq!(
            extract_portal_error(html).as_deref(),
            Some("T.C. Kimlik No veya İlk Şifre Hatalı!")
        );
    }

    #[test]
    fn test_extract_portal_success() {
        let html = r#"<div class="ui-messages-info ui-corner-all"><span class="ui-messages-info-detail">Parolanız başarıyla güncellendi</span></div>"#;
        assert_eq!(
            extract_portal_success(html).as_deref(),
            Some("Parolanız başarıyla güncellendi")
        );
    }

    #[test]
    fn test_extract_button_ids() {
        let kayit_html = r#"<button id="takeFirstInfoForm:j_idt45" name="takeFirstInfoForm:j_idt45" class="btn" type="submit"><span class="ui-button-text">Continue</span></button><button id="takeFirstInfoForm:j_idt47" class="btn" type="submit"><span class="ui-button-text">Back</span></button>"#;
        assert_eq!(
            extract_register_button_id(kayit_html).as_deref(),
            Some("takeFirstInfoForm:j_idt45")
        );

        let forgot_html = r#"<button id="passwordForgetForm:j_idt36" name="passwordForgetForm:j_idt36" class="btn" type="submit"><span class="ui-button-text">Update Password</span></button><button id="passwordForgetForm:j_idt38" class="btn" type="submit"><span class="ui-button-text">Back</span></button>"#;
        assert_eq!(
            extract_forgot_password_button_id(forgot_html).as_deref(),
            Some("passwordForgetForm:j_idt36")
        );

        let change_html = r#"<button id="passwordUpdateForm:j_idt43" name="passwordUpdateForm:j_idt43" class="btn" type="submit"><span class="ui-button-text">Save</span></button><button id="passwordUpdateForm:j_idt45" class="btn" type="submit"><span class="ui-button-text">Main Page</span></button>"#;
        assert_eq!(
            extract_change_password_button_id(change_html).as_deref(),
            Some("passwordUpdateForm:j_idt43")
        );
    }

    #[test]
    fn test_parse_turkish_quota_page() {
        let html = r#"
        <span class="myinfo">ÖRNEK ÖĞRENCİ </span>
        <table>
          <tr><td><label>Oturum Süresi:</label></td><td><label>0 gn 0 sa 12 dk 5 sn</label></td></tr>
          <tr><td><label>Login Zamanı:</label></td><td><label>14/10/2026 23:50:07</label></td></tr>
        </table>
        <table class="ui-panelgrid ui-widget panelGrid">
          <tbody>
            <tr><td><label>Paket Tipi:</label></td><td><span>Social Media</span></td></tr>
            <tr><td><label>Toplam Kota (MB):</label></td><td><label>5120.0</label></td></tr>
            <tr><td><label>Toplam Kalan Kota (MB):</label></td><td><label>5120.0</label></td></tr>
            <tr><td><label>Başlangıç Tarihi:</label></td><td><label>01.10.2026</label></td></tr>
            <tr><td><label>Sona Erme Tarihi:</label></td><td><label>31.10.2026</label></td></tr>
            <tr><td><label>Kalan Kota Zamanı:</label></td><td><label>17 gn 0 sa 9 dk 52 sn</label></td></tr>
          </tbody>
        </table>
        <table class="ui-panelgrid ui-widget panelGrid">
          <tbody>
            <tr><td><label>Paket Tipi:</label></td><td><span>Total</span></td></tr>
            <tr><td><label>Toplam Kota (MB):</label></td><td><label>32768.0</label></td></tr>
            <tr><td><label>Toplam Kalan Kota (MB):</label></td><td><label>15187.7</label></td></tr>
            <tr><td><label>Başlangıç Tarihi:</label></td><td><label>01.10.2026</label></td></tr>
            <tr><td><label>Sona Erme Tarihi:</label></td><td><label>31.10.2026</label></td></tr>
            <tr><td><label>Kalan Kota Zamanı:</label></td><td><label>17 gn 0 sa 9 dk 52 sn</label></td></tr>
          </tbody>
        </table>
        "#;
        let info = parse_quota_page(html).unwrap();
        assert_eq!(info.full_name.as_deref(), Some("ÖRNEK ÖĞRENCİ"));
        assert_eq!(info.session_time.as_deref(), Some("0 gn 0 sa 12 dk 5 sn"));
        assert_eq!(info.login_time.as_deref(), Some("14/10/2026 23:50:07"));
        assert_eq!(info.packages.len(), 2);
        assert_eq!(info.packages[0].total_mb, 5120.0);
        assert_eq!(info.packages[1].remaining_mb, 15187.7);
    }

    #[test]
    fn test_parse_faq_page() {
        let html = r#"
        <ol id="list3">
          <li>
            <p class="myQuestion">Kullanıcı adı ve şifremi nasıl alabilirim ?</p>
            <p class="myAnswer">Kullanıcı adı ve şifrenizi yurt idaresinden alabilirsiniz.</p>
          </li>
        </ol>
        "#;
        let items = parse_faq_page(html);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].question,
            "Kullanıcı adı ve şifremi nasıl alabilirim ?"
        );
    }

    #[test]
    fn test_is_max_sessions_message() {
        assert!(is_max_sessions_message("Maksimum oturum sayısına ulaşıldı"));
        assert!(is_max_sessions_message(
            "Aynı anda tek bir cihazdan oturum açabilirsiniz"
        ));
        assert!(is_max_sessions_message(
            "Mevcut bir oturumunuz bulunmaktadır"
        ));
        assert!(!is_max_sessions_message("Kullanıcı adı veya şifre hatalı"));
    }

    #[test]
    fn test_parse_last_actions_unauthenticated() {
        let login_html =
            r#"<html><form action="j_spring_security_check" method="POST"></form></html>"#;
        let res = parse_last_actions(login_html);
        assert!(matches!(res, Err(PortalError::AuthenticationFailed(_))));

        let expired_html = r#"<html><title>login.html</title><body>Giriş yapınız</body></html>"#;
        let res_expired = parse_last_actions(expired_html);
        assert!(matches!(
            res_expired,
            Err(PortalError::AuthenticationFailed(_))
        ));
    }

    #[test]
    fn test_parse_quota_page_with_login_error_myinfo() {
        let error_html = r#"
        <span class="myinfo" style="background-color:red">Kullanıcı adı veya şifre hatalı!</span>
        <form action="j_spring_security_check" method="POST"></form>
        "#;
        let res = parse_quota_page(error_html);
        match res {
            Err(PortalError::AuthenticationFailed(msg)) => {
                assert_eq!(msg, "Kullanıcı adı veya şifre hatalı!");
            }
            other => panic!("Beklenmeyen sonuç: {other:?}"),
        }
    }

    #[test]
    fn test_parse_quota_page_unauthenticated_rejects() {
        let invalid_html = r#"<html><body><p>Bilinmeyen sayfa içeriği</p></body></html>"#;
        let res = parse_quota_page(invalid_html);
        assert!(matches!(res, Err(PortalError::ParseError(_))));
    }

    #[test]
    fn test_parse_quota_page_zero_packages_with_session() {
        let html = r#"
        <span class="myinfo">MEHMET KAYA</span>
        <form id="servisUpdateForm">
          <button id="servisUpdateForm:j_idt127">End Session</button>
        </form>
        "#;
        let res = parse_quota_page(html);
        assert!(res.is_ok());
        let info = res.unwrap();
        assert_eq!(info.full_name.as_deref(), Some("MEHMET KAYA"));
        assert!(info.packages.is_empty());
    }

    #[test]
    fn test_is_survey_page_negative() {
        let no_survey_html = r#"
        <div id="mainPanel:surveyTab">
            <form id="mainPanel:no_survey_main">
                <i><span id="mainPanel:no_survey_main:no_survey">There are no surveys.</span></i>
            </form>
        </div>
        <form id="servisUpdateForm"><button id="btn">End Session</button></form>
        "#;
        assert!(!is_survey_page(no_survey_html));
    }

    #[test]
    fn test_is_survey_page_positive() {
        let survey_html = r#"
        <div id="mainPanel:surveyTab">
            <form id="surveyForm" action="/index.html">
                <input type="radio" name="surveyForm:q1" value="1" />
                <input type="radio" name="surveyForm:q1" value="2" />
                <button id="surveyForm:submit" type="submit">Gönder</button>
            </form>
        </div>
        "#;
        assert!(is_survey_page(survey_html));
    }

    #[test]
    fn test_extract_and_build_survey_params() {
        let html = r#"
        <form id="mainPanel:tabview:surveyForm" action="/index.html" method="POST">
            <input type="hidden" name="javax.faces.ViewState" value="vs12345" />
            <input type="radio" name="surveyForm:soru1" value="1" />
            <input type="radio" name="surveyForm:soru1" value="2" />
            <input type="radio" name="surveyForm:soru1" value="3" />
            <input type="radio" name="surveyForm:soru2" value="Evet" />
            <input type="radio" name="surveyForm:soru2" value="Hayır" />
            <input type="text" name="surveyForm:gorus" />
            <button id="surveyForm:btnGonder" type="submit"><span>Anketi Tamamla</span></button>
        </form>
        "#;

        let form = extract_survey_form(html).expect("Anket formu çıkarılabilmeli");
        assert_eq!(form.view_state, "vs12345");
        assert_eq!(form.radio_groups.len(), 2);
        assert_eq!(form.text_inputs.len(), 1);
        assert_eq!(form.submit_button_id, "surveyForm:btnGonder");

        let params = build_random_survey_params(&form);
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "javax.faces.partial.ajax" && v == "true")
        );
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "surveyForm:soru1" && (v == "1" || v == "2" || v == "3"))
        );
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "surveyForm:soru2" && (v == "Evet" || v == "Hayır"))
        );
        assert!(params.iter().any(|(k, _)| k == "surveyForm:gorus"));
    }

    #[test]
    fn test_extract_survey_form_with_remind_later_button() {
        let html = r#"
        <form id="j_idt32:j_idt37:0:surveyForm" name="j_idt32:j_idt37:0:surveyForm" method="post" action="/surveyRequired.html">
            <input type="hidden" name="j_idt32:j_idt37:0:surveyForm" value="j_idt32:j_idt37:0:surveyForm" />
            <input type="hidden" name="javax.faces.ViewState" value="vs_survey_999" />
            <table id="j_idt32:j_idt37:0:surveyForm:j_idt41:0:j_idt46">
                <input name="j_idt32:j_idt37:0:surveyForm:j_idt41:0:j_idt46" type="radio" value="10160" />
                <input name="j_idt32:j_idt37:0:surveyForm:j_idt41:0:j_idt46" type="radio" value="10161" />
            </table>
            <input id="j_idt32:j_idt37:0:surveyForm:iletisimBilgi_input" name="j_idt32:j_idt37:0:surveyForm:iletisimBilgi_input" type="checkbox" />
            <button id="j_idt32:j_idt37:0:surveyForm:j_idt66" type="submit"><span>Kaydet ve Devam Et</span></button>
            <button id="j_idt32:j_idt37:0:surveyForm:j_idt67" type="submit"><span>Daha Sonra Hatırlat</span></button>
        </form>
        "#;

        assert!(is_survey_page(html));
        let form = extract_survey_form(html).expect("Gerçek anket formu ayrıştırılabilmeli");
        assert_eq!(form.form_id, "j_idt32:j_idt37:0:surveyForm");
        assert_eq!(form.action, "/surveyRequired.html");
        assert_eq!(form.view_state, "vs_survey_999");
        assert_eq!(
            form.submit_button_id,
            "j_idt32:j_idt37:0:surveyForm:j_idt67"
        );
        assert_eq!(form.radio_groups.len(), 1);
        assert_eq!(form.checkboxes.len(), 1);

        let params = build_random_survey_params(&form);
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "j_idt32:j_idt37:0:surveyForm:j_idt67"
                    && v == "j_idt32:j_idt37:0:surveyForm:j_idt67")
        );
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "j_idt32:j_idt37:0:surveyForm:iletisimBilgi_input" && v == "on")
        );
    }
}
