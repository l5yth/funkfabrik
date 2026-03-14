// Copyright (c) 2006-2026 afri & veit
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, Redirect},
    routing::get,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tera::{Context, Tera};
use tower_http::services::ServeDir;

const PAGES: &[(&str, &str)] = &[
    ("100", "Startseite"),
    ("101", "Radio hören"),
    ("170", "Wettermagazin"),
    ("300", "Fanseite"),
    ("666", "Kontakt"),
    ("777", "Spiele"),
    ("999", "Impressum"),
];

#[derive(Clone)]
struct AppState {
    tera: Arc<Tera>,
    http: reqwest::Client,
}

#[tokio::main]
async fn main() {
    let tera = Tera::new("templates/**/*.html").expect("failed to parse templates");
    let state = AppState {
        tera: Arc::new(tera),
        http: reqwest::Client::new(),
    };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/100") }))
        .route("/api/rss", get(rss_proxy))
        .route("/{page}", get(page_handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("FUNKFABRIK*B listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn page_handler(Path(page): Path<String>, State(state): State<AppState>) -> Html<String> {
    let pages: Vec<serde_json::Value> = PAGES
        .iter()
        .map(|(num, title)| serde_json::json!({"num": num, "title": title}))
        .collect();

    let page_title = PAGES
        .iter()
        .find(|(num, _)| *num == page)
        .map(|(_, title)| *title)
        .unwrap_or("???");

    let mut ctx = Context::new();
    ctx.insert("current_page", &page);
    ctx.insert("page_title", page_title);
    ctx.insert("pages", &pages);

    if page == "170" {
        let weather: String = match state
            .http
            .get("https://wttr.in/Berlin?format=2")
            .header("User-Agent", "curl/8.0")
            .send()
            .await
        {
            Ok(r) => r.text().await.unwrap_or_else(|_| "Wetterdaten nicht verfügbar".into()),
            Err(_) => "Wetterdaten nicht verfügbar".into(),
        };
        ctx.insert("weather", weather.trim());

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Epoch (1970-01-01) was a Thursday → weekday = (days + 3) % 7, Mon = 0
        let today = (now_secs / 86400 + 3) % 7;
        const DAYS: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
        const ICONS: [&str; 7] = ["☀", "🌤", "⛅", "🌦", "☁", "🌧", "⛈"];
        const COLORS: [&str; 4] = ["color-green", "color-yellow", "color-cyan", "color-red"];

        let mut seed = now_secs;
        let mut rng = move || -> u64 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            seed >> 33
        };

        let forecast: Vec<serde_json::Value> = (1u64..=3)
            .map(|i| {
                let day   = DAYS[((today + i) % 7) as usize];
                let icon  = ICONS[rng() as usize % ICONS.len()];
                let temp  = 8 + rng() as usize % 15;   // 8–22 °C
                let wind  = 5 + rng() as usize % 36;   // 5–40 km/h
                let color = COLORS[rng() as usize % COLORS.len()];
                serde_json::json!({ "day": day, "icon": icon, "temp": temp, "wind": wind, "color": color })
            })
            .collect();

        ctx.insert("forecast", &forecast);
    }

    let template = format!("{}.html", page);
    let html = state.tera.render(&template, &ctx).unwrap_or_else(|_| {
        ctx.insert("current_page", &page);
        state
            .tera
            .render("404.html", &ctx)
            .unwrap_or_else(|_| "<h1 style='color:#FC0204'>PAGE NOT FOUND</h1>".into())
    });

    Html(html)
}

async fn rss_proxy(
    State(state): State<AppState>,
) -> Result<(HeaderMap, String), StatusCode> {
    let body = state
        .http
        .get("https://archiv.funkfabrik-b.de/rss")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .text()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/rss+xml; charset=utf-8"));
    Ok((headers, body))
}
