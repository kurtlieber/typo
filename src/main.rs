//! typo — a native, Typora-style markdown editor for Omarchy.

#[allow(dead_code)] // Edit operations are wired into the editor in phase 3.
mod bus;
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
mod portal;
mod source;
mod qt;
mod session;
mod strut;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};

fn main() {
    cxx_qt::init_crate!(typo);
    cxx_qt::init_qml_module!("Typo");

    // Before the application exists, so Qt registers with the host portal
    // under this app id instead of registering twice.
    qt::glue::ffi::set_desktop_file_name(&QString::from("typo"));
    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        // Settings live in ~/.config/typo/typo.conf.
        app.as_mut().set_organization_name(&QString::from("typo"));
        app.as_mut().set_application_name(&QString::from("typo"));
        app.as_mut().set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }
    qt::glue::ffi::add_strut_fonts();
    // Fusion takes every colour from the palette, which the window fills
    // from the Omarchy theme.
    QQuickStyle::set_style(&QString::from("Fusion"));

    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        qt::glue::ffi::add_math_provider(engine.as_mut());
        qt::glue::ffi::add_diagram_provider(engine.as_mut());
        engine.load(&QUrl::from("qrc:/qt/qml/Typo/qml/Main.qml"));
    }
    // A broken QML file leaves no window; Qt has already printed why.
    if !engine.as_ref().is_some_and(qt::glue::ffi::has_roots) {
        std::process::exit(1);
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
