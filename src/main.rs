//! typo — a native, Typora-style markdown editor for Omarchy.

mod code;
mod diagram;
mod eqnum;
mod export;
mod file;
mod folder;
mod htmlview;
mod image;
mod math;
mod md;
mod palette;
mod physics;
mod platform;
mod source;
mod qt;
mod session;
mod strut;

use cxx_qt_lib::{QGuiApplication, QQuickStyle, QString};

fn main() {
    cxx_qt::init_crate!(typo);
    cxx_qt::init_qml_module!("Typo");

    // Before the application exists, so Qt registers with the host portal
    // under this app id instead of registering twice.
    qt::glue::ffi::set_desktop_file_name(&QString::from("typo"));
    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        // Settings live in ~/.config/typo/typo.conf (typo.ini on the Mac).
        #[cfg(target_os = "macos")]
        qt::glue::ffi::use_ini_settings();
        app.as_mut().set_organization_name(&QString::from("typo"));
        app.as_mut().set_application_name(&QString::from("typo"));
        app.as_mut().set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }
    qt::glue::ffi::add_strut_fonts();
    // Fusion takes every colour from the palette, which the window fills
    // from the Omarchy theme.
    QQuickStyle::set_style(&QString::from("Fusion"));

    // A broken QML file leaves no window; Qt has already printed why.
    if !qt::glue::ffi::open_first_window() {
        std::process::exit(1);
    }
    qt::glue::ffi::open_file_events();
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
