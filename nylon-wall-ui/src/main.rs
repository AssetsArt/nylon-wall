mod api_client;
mod app;
mod components;
mod models;
pub mod theme;
pub mod ws_client;

use app::App;

fn main() {
    dioxus::launch(App);
}
