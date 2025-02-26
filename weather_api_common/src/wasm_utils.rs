use anyhow::{format_err, Error};
use http::Method;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use url::Url;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Request, RequestInit, Response};

use weather_util_rust::{
    latitude::Latitude, longitude::Longitude, weather_api::WeatherLocation,
    weather_data::WeatherData, weather_forecast::WeatherForecast, ApiStringType,
};

use crate::WeatherEntry;

static API_ENDPOINT: &str = "https://cloud.ddboline.net/weather/";

pub async fn get_ip_address() -> Result<Ipv4Addr, JsValue> {
    let url: Url = "https://ipinfo.io/ip".parse().map_err(|e| {
        error!("error {e}");
        let e: JsValue = format!("{e}").into();
        e
    })?;
    let resp = text_fetch(&url, Method::GET).await?;
    let resp = resp
        .as_string()
        .ok_or_else(|| JsValue::from_str("Failed to get ip"))?
        .trim()
        .to_string();
    debug!("got resp {resp}");
    resp.parse().map_err(|e| {
        let e: JsValue = format!("{e}").into();
        e
    })
}

pub async fn get_location_from_ip(ip: Ipv4Addr) -> Result<WeatherLocation, JsValue> {
    #[derive(Default, Serialize, Deserialize)]
    struct Location {
        latitude: Latitude,
        longitude: Longitude,
    }

    let ipaddr = ip.to_string();
    let url = Url::parse("https://ipwhois.app/json/")
        .map_err(|e| {
            error!("error {e}");
            let e: JsValue = format!("{e}").into();
            e
        })?
        .join(&ipaddr)
        .map_err(|e| {
            error!("error {e}");
            let e: JsValue = format!("{e}").into();
            e
        })?;
    debug!("url {url}");
    let json = js_fetch(&url, Method::GET).await?;
    let location: Location = serde_wasm_bindgen::from_value(json)?;
    Ok(WeatherLocation::from_lat_lon(
        location.latitude,
        location.longitude,
    ))
}

pub async fn js_fetch(url: &Url, method: Method) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method(method.as_str());

    let request = Request::new_with_str_and_init(url.as_str(), &opts)?;
    let window = web_sys::window().unwrap();
    let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into().unwrap();
    JsFuture::from(resp.json()?).await
}

pub async fn text_fetch(url: &Url, method: Method) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method(method.as_str());

    let request = Request::new_with_str_and_init(url.as_str(), &opts)?;
    let window = web_sys::window().unwrap();
    let resp = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into().unwrap();
    JsFuture::from(resp.text()?).await
}

pub async fn get_weather_data_forecast(location: &WeatherLocation) -> WeatherEntry {
    debug!("{location:?}");
    let weather = get_weather_data(location).await.ok();
    let forecast = get_weather_forecast(location).await.ok();
    WeatherEntry { weather, forecast }
}

pub async fn get_weather_data(loc: &WeatherLocation) -> Result<WeatherData, Error> {
    let options = loc.get_options();
    run_api("weather", &options).await
}

pub async fn get_weather_forecast(loc: &WeatherLocation) -> Result<WeatherForecast, Error> {
    let options = loc.get_options();
    run_api("forecast", &options).await
}

pub async fn run_api<T: serde::de::DeserializeOwned>(
    command: &str,
    options: &[(&'static str, ApiStringType)],
) -> Result<T, Error> {
    let base_url = format!("{API_ENDPOINT}{command}");
    let url = Url::parse_with_params(&base_url, options)?;
    let json = js_fetch(&url, Method::GET)
        .await
        .map_err(|e| format_err!("{:?}", e))?;
    serde_wasm_bindgen::from_value(json).map_err(|e| format_err!("{:?}", e))
}

pub fn set_history(history: &[String]) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window"))?;
    let local_storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("No local storage"))?;
    let history_str = serde_json::to_string(history).map_err(|e| {
        let e: JsValue = format!("{e}").into();
        e
    })?;
    local_storage.set_item("history", &history_str)?;
    Ok(())
}

pub fn get_history() -> Result<Vec<String>, JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("No window"))?;
    let local_storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("No local storage"))?;
    match local_storage.get_item("history")? {
        Some(s) => serde_json::from_str(&s).map_err(|e| {
            let e: JsValue = format!("{e}").into();
            e
        }),
        None => Ok(vec![String::from("zip=10001")]),
    }
}
