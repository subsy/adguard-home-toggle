use crate::icons;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, CssProvider, Label, Box as GtkBox, Orientation};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

const CSS: &str = "
window {
    background: transparent;
}
.osd-container {
    background: rgba(15, 15, 20, 0.88);
    border-radius: 20px;
    padding: 32px 48px;
    border: 1px solid rgba(255, 255, 255, 0.08);
}
.osd-label {
    color: rgba(255, 255, 255, 0.92);
    font-size: 18px;
    font-weight: 500;
    letter-spacing: 0.5px;
}
.osd-sublabel {
    color: rgba(255, 255, 255, 0.5);
    font-size: 13px;
    font-weight: 400;
}
";

pub fn show(enabled: bool, subtitle: &str) {
    let subtitle = subtitle.to_string();
    let app = Application::builder()
        .application_id("com.adguard.toggle.osd")
        .build();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        let css = CssProvider::new();
        css.load_from_data(CSS);
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().unwrap(),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let container = GtkBox::new(Orientation::Vertical, 16);
        container.add_css_class("osd-container");
        container.set_halign(Align::Center);
        container.set_valign(Align::Center);

        let svg_data = if enabled { icons::shield_on_svg() } else { icons::shield_off_svg() };
        let stream = gtk4::gio::MemoryInputStream::from_bytes(&glib::Bytes::from(svg_data.as_bytes()));
        if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk4::gio::Cancellable::NONE) {
            let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
            let image = gtk4::Picture::for_paintable(&texture);
            image.set_size_request(64, 64);
            image.set_halign(Align::Center);
            container.append(&image);
        }

        let title = if enabled { "Protection On" } else { "Protection Off" };
        let title_label = Label::new(Some(title));
        title_label.add_css_class("osd-label");
        container.append(&title_label);

        if !subtitle.is_empty() {
            let sub = Label::new(Some(&subtitle));
            sub.add_css_class("osd-sublabel");
            container.append(&sub);
        }

        window.set_child(Some(&container));
        window.present();

        let app_clone = app.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            app_clone.quit();
        });
    });

    app.run_with_args::<&str>(&[]);
}
