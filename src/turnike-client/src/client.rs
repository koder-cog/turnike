use crate::endpoints::{
    PORTAL_BASE, PORTAL_CHANGE_PASSWORD, PORTAL_FAQ, PORTAL_FORGOT_PASSWORD, PORTAL_INDEX_PAGE,
    PORTAL_LAST_ACTIONS, PORTAL_LOGIN_CHECK, PORTAL_LOGIN_PAGE, PORTAL_REGISTER, PROBE_FIREFOX,
    PROBE_GSTATIC,
};
use crate::scraper::{
    extract_ajax_redirect, extract_change_password_button_id, extract_forgot_password_button_id,
    extract_form_submit_button_id, extract_logout_button_id, extract_portal_error,
    extract_portal_success, extract_register_button_id, extract_ui_message, extract_view_state,
    is_max_sessions_message, parse_faq_page, parse_last_actions, parse_quota_page,
};
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, LOCATION, ORIGIN, REFERER, USER_AGENT,
};
use reqwest::redirect::Policy;
use reqwest::{Client, Response, Url};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};
use turnike_types::{
    Credentials, FaqItem, LastActionItem, PasswordResetRequest, PortalError, QuotaInfo,
    RegistrationRequest, SessionState,
};

const BROWSER_UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0";

#[derive(Clone, Debug)]
pub struct PortalClient {
    http: Arc<RwLock<Client>>,
    probe_client: Client,
}

impl PortalClient {
    fn build_http_client() -> Result<Client, PortalError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(USER_AGENT, HeaderValue::from_static(BROWSER_UA));
        default_headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        default_headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7"),
        );

        Client::builder()
            .cookie_store(true)
            .default_headers(default_headers)
            .timeout(Duration::from_secs(12))
            .redirect(Policy::none())
            .build()
            .map_err(|e| PortalError::Network(e.to_string()))
    }

    pub fn new() -> Result<Self, PortalError> {
        let http = Self::build_http_client()?;

        let probe_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(Policy::none())
            .build()
            .map_err(|e| PortalError::Network(e.to_string()))?;

        Ok(Self {
            http: Arc::new(RwLock::new(http)),
            probe_client,
        })
    }

    fn client(&self) -> Client {
        self.http
            .read()
            .expect("HTTP istemci kilidi okunamadı")
            .clone()
    }

    // F5 BIG-IP yük dengeleyicisi oturum sürekliliğini BIGipServerpool_aps_portal
    // çerezi ve JSESSIONID ile sağlar. Eski oturum çerezleri veya açık TCP bağlantıları
    // kaldığında yeni kimlik doğrulama isteği eski sunucu havuzuna yönlendirilir
    // ve oturum devralma kilitlenir. reqwest çerez havuzunu seçici boşaltacak genel
    // bir API sunmadığı için istemciyi baştan oluşturmak çerez ve TLS önbelleğini sıfırlar.
    fn reset_cookies(&self) {
        if let Ok(fresh) = Self::build_http_client()
            && let Ok(mut guard) = self.http.write()
        {
            *guard = fresh;
        }
    }

    pub async fn check_internet(&self) -> Result<bool, PortalError> {
        let resp = self
            .probe_client
            .get(PROBE_FIREFOX)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        if resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            Ok(body.trim() == "success")
        } else {
            Ok(false)
        }
    }

    pub async fn check_gstatic_probe(&self) -> Result<bool, PortalError> {
        let resp = self
            .probe_client
            .get(PROBE_GSTATIC)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        Ok(resp.status().as_u16() == 204)
    }

    // F5 BIG-IP HTTPS trafiğini dışarıda sonlandırıp Tomcat'e düz HTTP aktarır.
    // Tomcat RemoteIpValve eksikliği nedeniyle 302 yanıtlarında açık http:// adresi döner.
    // reqwest otomatik yönlendirme politikası hedef URL'yi mutasyona uğratmaya izin vermediği
    // için yönlendirme Policy::none ile durdurulup HTTPS şemasına zorlanır.
    fn normalize_portal_location(location: &str) -> String {
        let trimmed = location.trim();
        let Ok(base) = Url::parse(PORTAL_BASE) else {
            return PORTAL_INDEX_PAGE.to_string();
        };

        if let Ok(mut parsed) = base.join(trimmed) {
            if parsed.host_str() == Some("wifi.gsb.gov.tr") && parsed.scheme() == "http" {
                let _ = parsed.set_scheme("https");
            }
            if parsed.path() == "/" || parsed.path().is_empty() {
                parsed.set_path("/index.html");
            }
            parsed.to_string()
        } else {
            PORTAL_INDEX_PAGE.to_string()
        }
    }

    async fn follow_https_redirects(
        &self,
        mut resp: Response,
    ) -> Result<(String, String), PortalError> {
        let http = self.client();
        let mut current_url = resp.url().to_string();

        for _ in 0..6 {
            if !resp.status().is_redirection() {
                break;
            }

            let Some(loc_header) = resp.headers().get(LOCATION) else {
                break;
            };
            let Ok(loc_str) = loc_header.to_str() else {
                break;
            };

            let next_url = Self::normalize_portal_location(loc_str);
            let _ = resp.bytes().await;

            debug!("Portal yönlendirmesi HTTPS üzerinden takip ediliyor: {next_url}");
            current_url = next_url.clone();
            resp = http
                .get(&next_url)
                .header(REFERER, PORTAL_BASE)
                .send()
                .await
                .map_err(|e| PortalError::Network(e.to_string()))?;
        }

        let body = resp
            .text()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        Ok((current_url, body))
    }

    pub async fn get_status(&self) -> Result<SessionState, PortalError> {
        let http = self.client();
        let resp = match http.get(PORTAL_INDEX_PAGE).send().await {
            Ok(r) => r,
            Err(e) => {
                debug!("Portal sunucusuna erişilemedi: {e}");
                return Ok(SessionState::Offline);
            }
        };

        if resp.status().is_redirection()
            && let Some(loc) = resp.headers().get(LOCATION).and_then(|v| v.to_str().ok())
            && loc.contains("login.html")
        {
            let _ = resp.bytes().await;
            return Ok(SessionState::CaptiveLocked);
        }

        let (final_url, body) = match self.follow_https_redirects(resp).await {
            Ok(pair) => pair,
            Err(e) => {
                debug!("Portal yönlendirmesi tamamlanamadı: {e}");
                return Ok(SessionState::CaptiveLocked);
            }
        };

        if final_url.contains("login.html") || body.contains("j_spring_security_check") {
            return Ok(SessionState::CaptiveLocked);
        }

        match parse_quota_page(&body) {
            Ok(quota) => Ok(SessionState::Authenticated(quota)),
            Err(e) => {
                warn!("Kota bilgileri ayrıştırılamadı: {e}");
                Ok(SessionState::CaptiveLocked)
            }
        }
    }

    pub async fn login(&self, creds: &Credentials) -> Result<QuotaInfo, PortalError> {
        info!(
            "Portal giriş denemesi başlatılıyor: {}",
            creds.masked_username()
        );

        self.reset_cookies();
        let http = self.client();

        let mut submit_val = "Login";
        if let Ok(pre_resp) = http.get(PORTAL_LOGIN_PAGE).send().await
            && let Ok(login_html) = pre_resp.text().await
            && login_html.contains("value=\"Giriş\"")
        {
            submit_val = "Giriş";
        }

        let params = [
            ("j_username", creds.username.as_str()),
            ("j_password", creds.password.as_str()),
            ("submit", submit_val),
        ];

        let resp = http
            .post(PORTAL_LOGIN_CHECK)
            .header(REFERER, PORTAL_LOGIN_PAGE)
            .header(ORIGIN, PORTAL_BASE)
            .form(&params)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        if resp.status().is_redirection()
            && let Some(loc) = resp.headers().get(LOCATION).and_then(|v| v.to_str().ok())
            && (loc.contains("login.html?error") || loc.contains("error=true"))
        {
            let err_url = Self::normalize_portal_location(loc);
            let _ = resp.bytes().await;
            let err_body = match http.get(&err_url).send().await {
                Ok(r) => r.text().await.unwrap_or_default(),
                Err(_) => String::new(),
            };
            let error_msg = extract_portal_error(&err_body)
                .or_else(|| extract_ui_message(&err_body))
                .unwrap_or_else(|| "Kullanıcı adı veya şifre hatalı".to_string());
            if is_max_sessions_message(&error_msg) {
                return Err(PortalError::MaxSessionsReached(error_msg));
            }
            return Err(PortalError::AuthenticationFailed(error_msg));
        }

        let (final_url, body) = self.follow_https_redirects(resp).await?;

        if final_url.contains("login.html?error") || final_url.contains("error=true") {
            let error_msg = extract_portal_error(&body)
                .or_else(|| extract_ui_message(&body))
                .unwrap_or_else(|| "Kullanıcı adı veya şifre hatalı".to_string());
            if is_max_sessions_message(&error_msg) {
                return Err(PortalError::MaxSessionsReached(error_msg));
            }
            return Err(PortalError::AuthenticationFailed(error_msg));
        }

        let body = if final_url.contains("index.html")
            && !body.is_empty()
            && !body.contains("j_spring_security_check")
        {
            body
        } else {
            let index_resp = http
                .get(PORTAL_INDEX_PAGE)
                .header(REFERER, PORTAL_LOGIN_PAGE)
                .send()
                .await
                .map_err(|e| PortalError::Network(e.to_string()))?;
            let (_, index_body) = self.follow_https_redirects(index_resp).await?;
            index_body
        };

        if let Some(err) = extract_portal_error(&body) {
            if is_max_sessions_message(&err) {
                return Err(PortalError::MaxSessionsReached(err));
            }
            return Err(PortalError::AuthenticationFailed(err));
        }

        if body.contains("j_spring_security_check") || final_url.contains("login.html") {
            let error_msg = extract_ui_message(&body)
                .unwrap_or_else(|| "Kullanıcı adı veya şifre hatalı".to_string());
            if is_max_sessions_message(&error_msg) {
                return Err(PortalError::MaxSessionsReached(error_msg));
            }
            return Err(PortalError::AuthenticationFailed(error_msg));
        }

        match parse_quota_page(&body) {
            Ok(quota) => Ok(quota),
            Err(e) => {
                // Yalnızca sayfada servisUpdateForm veya çıkış butonu varsa (yani oturumun
                // kesinlikle açıldığı ama kota tablosunun henüz dolmadığı durumlarda) boş kota dönülür.
                if body.contains("servisUpdateForm") || extract_logout_button_id(&body).is_some() {
                    Ok(QuotaInfo {
                        packages: Vec::new(),
                        session_time: None,
                        login_time: None,
                        full_name: None,
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn change_password(
        &self,
        current_pass: &str,
        new_pass: &str,
    ) -> Result<(), PortalError> {
        info!("Portal şifre değiştirme işlemi başlatılıyor");
        let http = self.client();

        let page_resp = http
            .get(PORTAL_CHANGE_PASSWORD)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let (final_url, body) = self.follow_https_redirects(page_resp).await?;
        if final_url.contains("login.html") || body.contains("j_spring_security_check") {
            return Err(PortalError::AuthenticationFailed(
                "Şifre değiştirmek için aktif oturum bulunamadı. Lütfen önce giriş yapınız."
                    .to_string(),
            ));
        }

        let view_state = extract_view_state(&body)?;
        let button_id = extract_change_password_button_id(&body)
            .unwrap_or_else(|| "passwordUpdateForm:j_idt43".to_string());

        let params = [
            ("javax.faces.partial.ajax", "true"),
            ("javax.faces.source", button_id.as_str()),
            ("javax.faces.partial.execute", "@all"),
            ("javax.faces.partial.render", "passwordUpdateForm"),
            ("passwordUpdateForm", "passwordUpdateForm"),
            ("passwordUpdateForm:txtMevcutParola", current_pass),
            ("passwordUpdateForm:txtYeniParola", new_pass),
            ("passwordUpdateForm:txtParolaDogrulama", new_pass),
            (button_id.as_str(), button_id.as_str()),
            ("javax.faces.ViewState", view_state.as_str()),
        ];

        let resp = http
            .post(PORTAL_CHANGE_PASSWORD)
            .header(REFERER, PORTAL_CHANGE_PASSWORD)
            .header(ORIGIN, PORTAL_BASE)
            .header("Faces-Request", "partial/ajax")
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&params)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let resp_body = resp
            .text()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        if let Some(err) = extract_portal_error(&resp_body) {
            return Err(PortalError::AuthenticationFailed(err));
        }

        if let Some(redirect_url) = extract_ajax_redirect(&resp_body)
            && (redirect_url.contains("index") || redirect_url.contains("login"))
        {
            return Ok(());
        }

        if let Some(success) = extract_portal_success(&resp_body) {
            let lower = success.to_lowercase();
            if lower.contains("başarı")
                || lower.contains("success")
                || lower.contains("güncellen")
                || lower.contains("updated")
                || lower.contains("değiştiril")
            {
                return Ok(());
            }
        }

        let body_lower = resp_body.to_lowercase();
        if body_lower.contains("başarıyla")
            || body_lower.contains("successfully")
            || body_lower.contains("güncellendi")
            || body_lower.contains("updated")
            || (resp_body.contains("index.html") && !resp_body.contains("passwordUpdateForm"))
        {
            return Ok(());
        }

        Err(PortalError::AuthenticationFailed(
            "Şifre değiştirilemedi. Mevcut şifrenizi kontrol ediniz.".to_string(),
        ))
    }

    pub async fn reset_password_edevlet(
        &self,
        req: &PasswordResetRequest,
    ) -> Result<(), PortalError> {
        info!("e-Devlet geçici şifresi ile parola sıfırlama başlatılıyor");
        let http = self.client();

        let page_resp = http
            .get(PORTAL_FORGOT_PASSWORD)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let (_, body) = self.follow_https_redirects(page_resp).await?;
        let view_state = extract_view_state(&body)?;
        let button_id = extract_forgot_password_button_id(&body)
            .unwrap_or_else(|| "passwordForgetForm:j_idt36".to_string());

        let params = [
            ("javax.faces.partial.ajax", "true"),
            ("javax.faces.source", button_id.as_str()),
            ("javax.faces.partial.execute", "@all"),
            ("javax.faces.partial.render", "@all"),
            ("passwordForgetForm", "passwordForgetForm"),
            (
                "passwordForgetForm:usernameForUpdate",
                req.username.as_str(),
            ),
            (
                "passwordForgetForm:txtGeciciSifre",
                req.temporary_password.as_str(),
            ),
            (
                "passwordForgetForm:txtYeniParola",
                req.new_password.as_str(),
            ),
            (
                "passwordForgetForm:txtParolaDogrulama",
                req.new_password.as_str(),
            ),
            (button_id.as_str(), button_id.as_str()),
            ("javax.faces.ViewState", view_state.as_str()),
        ];

        let resp = http
            .post(PORTAL_FORGOT_PASSWORD)
            .header(REFERER, PORTAL_FORGOT_PASSWORD)
            .header(ORIGIN, PORTAL_BASE)
            .header("Faces-Request", "partial/ajax")
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&params)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let resp_body = resp
            .text()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        if let Some(err) = extract_portal_error(&resp_body) {
            return Err(PortalError::AuthenticationFailed(err));
        }

        if let Some(redirect_url) = extract_ajax_redirect(&resp_body)
            && (redirect_url.contains("login") || redirect_url.contains("index"))
        {
            return Ok(());
        }

        if let Some(success) = extract_portal_success(&resp_body) {
            let lower = success.to_lowercase();
            if lower.contains("başarı")
                || lower.contains("success")
                || lower.contains("güncellen")
                || lower.contains("updated")
            {
                return Ok(());
            }
        }

        let body_lower = resp_body.to_lowercase();
        if body_lower.contains("başarıyla")
            || body_lower.contains("successfully")
            || body_lower.contains("güncellendi")
            || body_lower.contains("updated")
            || (resp_body.contains("login.html") && !resp_body.contains("passwordForgetForm"))
        {
            return Ok(());
        }

        Err(PortalError::AuthenticationFailed(
            "Parola sıfırlanamadı. Bilgilerinizi ve e-Devlet geçici şifrenizin geçerliliğini kontrol ediniz.".to_string(),
        ))
    }

    pub async fn register_first_user(
        &self,
        req: &RegistrationRequest,
    ) -> Result<String, PortalError> {
        info!("Yeni kullanıcı ilk giriş doğrulaması başlatılıyor");
        let http = self.client();

        let page_resp = http
            .get(PORTAL_REGISTER)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let (_, body) = self.follow_https_redirects(page_resp).await?;
        let view_state = extract_view_state(&body)?;
        let button_id = extract_register_button_id(&body)
            .unwrap_or_else(|| "takeFirstInfoForm:j_idt45".to_string());

        let params = [
            ("javax.faces.partial.ajax", "true"),
            ("javax.faces.source", button_id.as_str()),
            ("javax.faces.partial.execute", "@all"),
            ("takeFirstInfoForm", "takeFirstInfoForm"),
            ("takeFirstInfoForm:uyrukRadio", req.nationality.form_value()),
            ("takeFirstInfoForm:cmbKullaniciTipi_focus", ""),
            (
                "takeFirstInfoForm:cmbKullaniciTipi_input",
                req.user_type.form_value(),
            ),
            ("takeFirstInfoForm:tcKimlikNo", req.identity_number.as_str()),
            (
                "takeFirstInfoForm:firstPassword",
                req.first_password.as_str(),
            ),
            (button_id.as_str(), button_id.as_str()),
            ("javax.faces.ViewState", view_state.as_str()),
        ];

        let resp = http
            .post(PORTAL_REGISTER)
            .header(REFERER, PORTAL_REGISTER)
            .header(ORIGIN, PORTAL_BASE)
            .header("Faces-Request", "partial/ajax")
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&params)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let resp_body = resp
            .text()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        if let Some(err) = extract_portal_error(&resp_body) {
            return Err(PortalError::AuthenticationFailed(err));
        }

        if let Some(redirect_url) = extract_ajax_redirect(&resp_body) {
            if redirect_url.contains("sifreDegistirme") {
                return Ok("İlk şifre doğrulandı, yeni şifrenizi belirleyebilirsiniz".to_string());
            }
            if redirect_url.contains("login") {
                return Err(PortalError::AuthenticationFailed(
                    "Oturum doğrulanamadı, giriş sayfasına yönlendirildi".to_string(),
                ));
            }
        }

        if resp_body.contains("sifreDegistirme") || resp_body.contains("passwordUpdateForm") {
            return Ok("İlk şifre doğrulandı, yeni şifrenizi belirleyebilirsiniz".to_string());
        }

        if let Some(success) = extract_portal_success(&resp_body) {
            return Ok(success);
        }

        Err(PortalError::AuthenticationFailed(
            "Kimlik bilgileri veya ilk şifre doğrulanamadı".to_string(),
        ))
    }

    pub async fn get_faq(&self) -> Result<Vec<FaqItem>, PortalError> {
        let http = self.client();
        let resp = http
            .get(PORTAL_FAQ)
            .header(REFERER, PORTAL_INDEX_PAGE)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let (_, body) = self.follow_https_redirects(resp).await?;
        Ok(parse_faq_page(&body))
    }

    pub async fn get_last_actions(&self) -> Result<Vec<LastActionItem>, PortalError> {
        info!("Kullanıcı son hareketler listesi sorgulanıyor");
        let http = self.client();

        let resp = http
            .get(PORTAL_LAST_ACTIONS)
            .header(REFERER, PORTAL_INDEX_PAGE)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let (_, body) = self.follow_https_redirects(resp).await?;

        parse_last_actions(&body)
    }

    pub async fn logout(&self) -> Result<(), PortalError> {
        info!("Portal oturumu sonlandırılıyor");
        let http = self.client();

        let index_resp = http
            .get(PORTAL_INDEX_PAGE)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        let (final_url, body) = self.follow_https_redirects(index_resp).await?;

        if final_url.contains("login.html") || body.contains("j_spring_security_check") {
            debug!("Zaten oturum açılmamış veya sonlanmış, çerezler sıfırlanıyor");
            self.reset_cookies();
            return Ok(());
        }

        let view_state = extract_view_state(&body)?;
        let button_id = extract_logout_button_id(&body)
            .or_else(|| extract_form_submit_button_id(&body, "servisUpdateForm"))
            .unwrap_or_else(|| "servisUpdateForm:j_idt127".to_string());

        // GSB portalı standart bir REST logout uç noktası barındırmaz. PrimeFaces 6.0
        // oturumu sonlandırmak için sayfadan alınan ViewState ve dinamik buton ID'siyle
        // kısmi AJAX postback çağrısını zorunlu kılar.
        let params = [
            ("javax.faces.partial.ajax", "true"),
            ("javax.faces.source", button_id.as_str()),
            ("javax.faces.partial.execute", "@all"),
            ("servisUpdateForm", "servisUpdateForm"),
            (&button_id, &button_id),
            ("javax.faces.ViewState", view_state.as_str()),
        ];

        let logout_resp = http
            .post(PORTAL_INDEX_PAGE)
            .header(REFERER, PORTAL_INDEX_PAGE)
            .header(ORIGIN, PORTAL_BASE)
            .header("Faces-Request", "partial/ajax")
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&params)
            .send()
            .await
            .map_err(|e| PortalError::Network(e.to_string()))?;

        debug!("Çıkış isteği durumu: {}", logout_resp.status());
        let resp_xml = logout_resp.text().await.unwrap_or_default();

        if let Some(err) = extract_portal_error(&resp_xml) {
            return Err(PortalError::AuthenticationFailed(err));
        }

        if let Some(redirect_target) = extract_ajax_redirect(&resp_xml) {
            let next_url = Self::normalize_portal_location(&redirect_target);
            if let Ok(r) = http
                .get(&next_url)
                .header(REFERER, PORTAL_INDEX_PAGE)
                .send()
                .await
            {
                let _ = self.follow_https_redirects(r).await;
            }
        }

        self.reset_cookies();
        Ok(())
    }

    pub async fn preempt_session(&self, creds: &Credentials) -> Result<QuotaInfo, PortalError> {
        info!(
            "Askıda kalan oturumu devralma süreci başlatılıyor: {}",
            creds.masked_username()
        );

        let _ = self.logout().await;

        let max_attempts = 5;
        let mut backoff = Duration::from_millis(300);

        for attempt in 1..=max_attempts {
            tokio::time::sleep(backoff).await;
            debug!("Oturum devralma denemesi yapılıyor: {attempt}/{max_attempts}");

            match self.login(creds).await {
                Ok(quota) => {
                    info!("Oturum {attempt}. denemede devralındı");
                    return Ok(quota);
                }
                Err(PortalError::MaxSessionsReached(msg)) => {
                    debug!("Önceki oturum henüz sonlanmadı: {msg}");
                    if attempt == max_attempts {
                        return Err(PortalError::MaxSessionsReached(msg));
                    }
                    backoff = (backoff * 3 / 2).min(Duration::from_millis(1000));
                }
                Err(other_err) => {
                    return Err(other_err);
                }
            }
        }

        Err(PortalError::AuthenticationFailed(
            "Oturum devralma denemeleri zaman aşımına uğradı".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_portal_location() {
        assert_eq!(
            PortalClient::normalize_portal_location("http://wifi.gsb.gov.tr/index.html"),
            "https://wifi.gsb.gov.tr/index.html"
        );
        assert_eq!(
            PortalClient::normalize_portal_location("http://wifi.gsb.gov.tr/"),
            "https://wifi.gsb.gov.tr/index.html"
        );
        assert_eq!(
            PortalClient::normalize_portal_location("/login.html"),
            "https://wifi.gsb.gov.tr/login.html"
        );
        assert_eq!(
            PortalClient::normalize_portal_location("login.html"),
            "https://wifi.gsb.gov.tr/login.html"
        );
        assert_eq!(
            PortalClient::normalize_portal_location("https://wifi.gsb.gov.tr"),
            "https://wifi.gsb.gov.tr/index.html"
        );
    }
}
