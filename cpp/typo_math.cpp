#include <QClipboard>
#include <QFontDatabase>
#include <QGuiApplication>
#include <QImage>
#include <QQuickImageProvider>

#include "typo/src/qt/glue.cxxqt.h"

namespace {

// Typeset math for QML: `image://math/<id>` is the formula `id` describes
// (see src/qt/math.rs), drawn by Rust on Qt's image-loading threads so a
// long document's formulas never hold up typing.
class MathProvider : public QQuickImageProvider {
public:
    MathProvider()
        : QQuickImageProvider(QQuickImageProvider::Image, QQmlImageProviderBase::ForceAsynchronousImageLoading)
    {
    }

    QImage requestImage(const QString &id, QSize *size, const QSize &) override
    {
        std::int32_t width = 0;
        std::int32_t height = 0;
        rust::Vec<std::uint8_t> rgba = typoRenderMath(id, width, height);
        // The pixels belong to the Rust vector, so the image takes a copy.
        QImage image = QImage(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888_Premultiplied).copy();
        if (size)
            *size = image.size();
        return image;
    }
};

// Diagrams for QML: `image://diagram/<id>` is the fence `id` describes (see
// src/qt/diagram.rs), drawn by Rust as a PNG on Qt's image-loading threads.
// A diagram that fails is one clear pixel.
class DiagramProvider : public QQuickImageProvider {
public:
    DiagramProvider()
        : QQuickImageProvider(QQuickImageProvider::Image, QQmlImageProviderBase::ForceAsynchronousImageLoading)
    {
    }

    QImage requestImage(const QString &id, QSize *size, const QSize &) override
    {
        rust::Vec<std::uint8_t> png = typoRenderDiagram(id);
        QImage image;
        if (!png.empty())
            image.loadFromData(png.data(), int(png.size()), "PNG");
        if (image.isNull()) {
            image = QImage(1, 1, QImage::Format_ARGB32_Premultiplied);
            image.fill(Qt::transparent);
        }
        if (size)
            *size = image.size();
        return image;
    }
};

} // namespace

void typoAddDiagramProvider(QQmlApplicationEngine &engine)
{
    engine.addImageProvider(QStringLiteral("diagram"), new DiagramProvider);
}

void typoAddMathProvider(QQmlApplicationEngine &engine)
{
    engine.addImageProvider(QStringLiteral("math"), new MathProvider);
}

bool typoCopyMathImage(const QString &id)
{
    std::int32_t width = 0;
    std::int32_t height = 0;
    rust::Vec<std::uint8_t> rgba = typoRenderMath(id, width, height);
    // A formula that failed is one clear pixel.
    if (width <= 1 && height <= 1)
        return false;
    QGuiApplication::clipboard()->setImage(QImage(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888_Premultiplied).copy());
    return true;
}

void typoAddStrutFonts()
{
    for (std::uint16_t n = 0;; ++n) {
        rust::Vec<std::uint8_t> font = typoStrutFont(n);
        if (font.empty())
            break;
        QFontDatabase::addApplicationFontFromData(QByteArray(reinterpret_cast<const char *>(font.data()), qsizetype(font.size())));
    }
}
