fn main() {
    env_logger::init();
    crate::ui::window::init_window();
}

mod graphics;
mod terminal;
mod ui;
