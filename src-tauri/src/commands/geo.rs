use std::net::IpAddr;
use std::time::Duration;
use tauri::{AppHandle, State};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoInfo {
    country_code: String,
    country_name: String,
    city_name: String,
    region_name: String,
}

fn public_ip(server: &str) -> Result<IpAddr, String> {
    let ip: IpAddr = server
        .parse()
        .map_err(|_| "Only public IP addresses can be looked up".to_string())?;
    let allowed = crate::config::subscription::is_public_ip(ip)
        && !matches!(ip, IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some());
    if allowed {
        Ok(ip)
    } else {
        Err("Only public IP addresses can be looked up".to_string())
    }
}

fn geo_lookup_consent(settings: &crate::state::UserSettings) -> Result<(), String> {
    if settings.geo_lookup_enabled {
        Ok(())
    } else {
        Err("Node geolocation is disabled in Settings".to_string())
    }
}

#[tauri::command]
pub async fn lookup_node_geo(
    app: AppHandle,
    state: State<'_, crate::state::ProxyState>,
    server: String,
) -> Result<GeoInfo, String> {
    geo_lookup_consent(&state.reload_settings(&app)?)?;
    let ip = public_ip(server.trim())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| "Could not start geolocation lookup".to_string())?;
    let mut response = client
        .get(format!("https://freeipapi.com/api/json/{ip}"))
        .send()
        .await
        .map_err(|_| "Geolocation service is unavailable".to_string())?;
    if !response.status().is_success() || response.content_length().is_some_and(|len| len > 4096) {
        return Err("Geolocation service returned an invalid response".to_string());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Geolocation service returned an invalid response".to_string())?
    {
        if chunk.len() > 4096 - body.len() {
            return Err("Geolocation response is too large".to_string());
        }
        body.extend_from_slice(&chunk);
    }
    let geo: GeoInfo = serde_json::from_slice(&body)
        .map_err(|_| "Geolocation response is malformed".to_string())?;
    if geo.country_code.len() != 2 || !geo.country_code.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err("Geolocation country code is invalid".to_string());
    }
    Ok(geo)
}

#[cfg(test)]
mod tests {
    use super::{geo_lookup_consent, public_ip};
    use crate::state::UserSettings;

    #[test]
    fn only_public_ip_is_eligible_for_geo_lookup() {
        for valid in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(public_ip(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "localhost",
            "example.com",
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.1",
            "172.16.0.1",
            "172.31.255.255",
            "100.64.0.1",
            "169.254.3.4",
            "192.0.0.1",
            "192.0.2.1",
            "192.88.99.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:8.8.8.8",
        ] {
            assert!(public_ip(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn consent_is_off_until_explicitly_enabled() {
        let mut settings = UserSettings::default();
        assert!(geo_lookup_consent(&settings).is_err());
        settings.geo_lookup_enabled = true;
        assert!(geo_lookup_consent(&settings).is_ok());
    }
}
