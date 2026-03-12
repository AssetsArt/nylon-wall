mod api_client;
mod app;
mod components;
mod models;

use app::App;

fn main() {
    dioxus::launch(App);
}
