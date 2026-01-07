use leptos::mount::mount_to_body;

mod page;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(page::app::App);
}
