mod api_client;
mod app;
mod components;
mod models;

fn main() {
    dioxus::launch(app::App);
}
