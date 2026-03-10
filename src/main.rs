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
    response::{Html, Redirect},
    routing::get,
};
use std::sync::Arc;
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
}

#[tokio::main]
async fn main() {
    let tera = Tera::new("templates/**/*.html").expect("failed to parse templates");
    let state = AppState {
        tera: Arc::new(tera),
    };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/100") }))
        .route("/:page", get(page_handler))
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
