//! Application-level Qt calls that cxx-qt-lib doesn't wrap.

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;
        include!("typo_glue.h");

        /// Ties windows to `<name>.desktop`, so the compositor's app id and
        /// the launcher's icon match the installed entry.
        #[cxx_name = "typoSetDesktopFileName"]
        fn set_desktop_file_name(name: &QString);

        /// Keep settings in an INI file under `$XDG_CONFIG_HOME` (as on
        /// Linux) rather than the Mac's preferences, which ignore it.
        #[cxx_name = "typoUseIniSettings"]
        fn use_ini_settings();

        /// Whether the engine created any root object, i.e. Main.qml loaded.
        #[cxx_name = "typoHasRoots"]
        fn has_roots(engine: &QQmlApplicationEngine) -> bool;

        /// Serves typeset math to QML as `image://math/…`.
        #[cxx_name = "typoAddMathProvider"]
        fn add_math_provider(engine: Pin<&mut QQmlApplicationEngine>);

        /// Open each file the Mac hands over as an event (a double-click in
        /// Finder, Open With, a drop on the Dock icon) with the window's
        /// `openFile`, as one named on the command line.
        #[cxx_name = "typoOpenFileEvents"]
        fn open_file_events(engine: Pin<&mut QQmlApplicationEngine>);

        /// Serves drawn diagrams to QML as `image://diagram/…`.
        #[cxx_name = "typoAddDiagramProvider"]
        fn add_diagram_provider(engine: Pin<&mut QQmlApplicationEngine>);

        /// File → Export → Image: premultiplied RGBA `width` × `height`
        /// pixels as a "png" or "jpg" file's bytes; empty when Qt can't.
        #[cxx_name = "typoEncodeImage"]
        fn encode_image(width: u32, height: u32, rgba: &[u8], format: &QString) -> Vec<u8>;

        /// Gives Qt the strut fonts (src/strut.rs); after the application
        /// exists, before any text is laid out.
        #[cxx_name = "typoAddStrutFonts"]
        fn add_strut_fonts();

        /// Whether the desktop asks for light colours (the Mac's
        /// Appearance); false when it doesn't say.
        #[cxx_name = "typoPrefersLight"]
        fn prefers_light() -> bool;
    }

    extern "Rust" {
        /// The pixels for `image://math/<id>`, called on a worker thread.
        #[cxx_name = "typoRenderMath"]
        fn render_math(id: &QString, width: &mut i32, height: &mut i32) -> Vec<u8>;

        /// The PNG for `image://diagram/<id>`, or nothing; on a worker thread.
        #[cxx_name = "typoRenderDiagram"]
        fn render_diagram(id: &QString) -> Vec<u8>;

        /// Strut font `n`, or nothing past the last.
        #[cxx_name = "typoStrutFont"]
        fn strut_font(n: u16) -> Vec<u8>;
    }
}

use super::diagram::render_diagram;
use super::math::render_math;

fn strut_font(n: u16) -> Vec<u8> {
    if n > crate::strut::STEPS { Vec::new() } else { crate::strut::font(n) }
}
