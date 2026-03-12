mod api_client;
mod app;
mod components;
mod models;
pub mod theme;

use app::App;

fn main() {
    dioxus::launch(App);
}
