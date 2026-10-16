#pragma once

#include "rust/cxx.h"

#include <QBuffer>
#include <QClipboard>
#include <QColor>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QImage>
#include <QPainter>
#include <QUrl>
#include <QGuiApplication>
#include <QKeySequence>
#include <QMimeData>
#include <QWheelEvent>
#include <QQmlApplicationEngine>
#include <QQuickWindow>
#include <QQuickTextDocument>
#include <QSyntaxHighlighter>
#include <QTextBlock>
#include <QTextBlockFormat>
#include <QTextCharFormat>
#include <QTextCursor>
#include <QTextDocument>
#include <QTextDocumentFragment>
#include <QFontMetricsF>
#include <QTextLayout>
#include <QAbstractTextDocumentLayout>
#include <QtTest/qtestkeyboard.h>
#include <QtTest/qtestmouse.h>

// The C++ seam between Qt's text engine and the Rust highlighters.
// QSyntaxHighlighter keeps highlightBlock, setFormat and currentBlock
// protected and works in terms of QTextCharFormat values, none of which cxx
// can pass across the bridge. This adapter exposes them as plain scalars;
// every decision about what to format lives in Rust.
class TypoHighlighterBase : public QSyntaxHighlighter {
    Q_OBJECT

public:
    explicit TypoHighlighterBase(QObject *parent = nullptr) : QSyntaxHighlighter(parent) {}

    void attachQuickDocument(QQuickTextDocument *doc) { setDocument(doc ? doc->textDocument() : nullptr); }
    void rehighlightAll() { rehighlight(); }
    int blockPosition() const { return currentBlock().position(); }
    int blockNumber() const { return currentBlock().blockNumber(); }
    // Qt re-highlights the next line when a line's state changes.
    void setBlockState(int state) { setCurrentBlockState(state); }
    QString documentText() const { return document() ? document()->toPlainText() : QString(); }
    int previousState() const { return previousBlockState(); }
    void clearUndo() {
        if (document())
            document()->clearUndoRedoStacks();
    }

    // Source Code Mode's lines are the document's paragraphs: the one
    // holding `pos`, its number, where it starts, and its box (wrapped
    // lines and all) in the document; and where line `line` starts.
    QTextBlock lineAt(int pos) const { return document() ? document()->findBlock(pos) : QTextBlock(); }
    int docLineNumber(int pos) const { return qMax(0, lineAt(pos).blockNumber()); }
    int docLineStart(int pos) const { return qMax(0, lineAt(pos).position()); }
    QRectF docLineRect(int pos) const {
        const QTextBlock b = lineAt(pos);
        return b.isValid() ? document()->documentLayout()->blockBoundingRect(b) : QRectF();
    }
    QString docLineText(int pos) const { return lineAt(pos).text(); }
    // The line after the one being highlighted.
    QString nextLineText() const { return currentBlock().next().text(); }
    // The colour the highlighter gave the character at `pos`, as
    // "#rrggbb" ("" for none): for the tests.
    QString docInkAt(int pos) const {
        const QTextBlock b = lineAt(pos);
        if (!b.isValid() || !b.layout())
            return QString();
        const int at = pos - b.position();
        for (const QTextLayout::FormatRange &r : b.layout()->formats())
            if (r.start <= at && at < r.start + r.length && r.format.hasProperty(QTextFormat::ForegroundBrush))
                return r.format.foreground().color().name();
        return QString();
    }
    // What the highlighter left at the end of the line before the one
    // holding `pos` (-1 at the start).
    int docStateBefore(int pos) const {
        const QTextBlock b = lineAt(pos).previous();
        return b.isValid() ? b.userState() : -1;
    }
    // Replace `start`..`end` of the text with `text` as an edit of its
    // own, or, with `join`, as part of the edit before it: Source Code
    // Mode's Replace All is one undo step.
    void replaceRange(int start, int end, const QString &text, bool join) {
        QTextDocument *doc = document();
        if (!doc)
            return;
        QTextCursor c(doc);
        if (join)
            c.joinPreviousEditBlock();
        else
            c.beginEditBlock();
        c.setPosition(start);
        c.setPosition(end, QTextCursor::KeepAnchor);
        c.insertText(text);
        c.endEditBlock();
    }
    int docLinePosition(int line) const {
        QTextDocument *doc = document();
        if (!doc)
            return 0;
        const QTextBlock b = doc->findBlockByNumber(line);
        return b.isValid() ? b.position() : doc->lastBlock().position();
    }

    // Give every paragraph of the document a proportional line height
    // (percent). Text blocks are created and replaced as the user types, so
    // this runs after each change; blocks that already match are skipped,
    // which keeps it cheap and stops the format change from recursing.
    // Qt's proportional line height scales the font's own height (about
    // 1.36em for Open Sans), not the em like CSS, so set pixels instead.
    // `minimum` lets a line grow past it, for inline images and formulas
    // taller than the text; Qt then sets the text at the bottom of the line
    // rather than at 4/5 of it, a fraction of a pixel lower.
    // Also indents the first line by `firstIndent` pixels (a footnote's
    // label sits there).
    void applyLineHeight(qreal pixels, bool minimum, qreal firstIndent) {
        QTextDocument *doc = document();
        if (!doc || pixels <= 0)
            return;
        const auto type = minimum ? QTextBlockFormat::MinimumHeight : QTextBlockFormat::FixedHeight;
        QTextCursor cursor(doc);
        bool editing = false;
        for (QTextBlock b = doc->begin(); b.isValid(); b = b.next()) {
            QTextBlockFormat f = b.blockFormat();
            const qreal indent = b == doc->begin() ? firstIndent : 0;
            if (f.lineHeightType() == type && qFuzzyCompare(f.lineHeight(), pixels) && qFuzzyCompare(f.textIndent() + 1, indent + 1))
                continue;
            if (!editing) {
                cursor.beginEditBlock();
                editing = true;
            }
            cursor.setPosition(b.position());
            f.setLineHeight(pixels, type);
            f.setTextIndent(indent);
            cursor.setBlockFormat(f);
        }
        if (editing)
            cursor.endEditBlock();
    }

    // flags: 1 bold, 2 italic, 4 underline, 8 strike, 16 mono, 32 hidden, 64 superscript, 128 subscript
    void applyFormat(int start, int count, quint32 flags, double pixelSize, quint32 fg, quint32 bg) {
        QTextCharFormat f;
        if (flags & 1) f.setFontWeight(QFont::Bold);
        if (flags & 2) f.setFontItalic(true);
        if (flags & 4) f.setFontUnderline(true);
        if (flags & 8) f.setFontStrikeOut(true);
        if (flags & 16) f.setFontFamilies({QStringLiteral("monospace")});
        if (flags & 64) f.setVerticalAlignment(QTextCharFormat::AlignSuperScript);
        if (flags & 128) f.setVerticalAlignment(QTextCharFormat::AlignSubScript);
        if (pixelSize > 0) f.setProperty(QTextFormat::FontPixelSize, pixelSize);
        if (fg) f.setForeground(QColor::fromRgba(fg));
        if (bg) f.setBackground(QColor::fromRgba(bg));
        if (flags & 32) {
            f.setProperty(QTextFormat::FontPixelSize, 1);
            f.setForeground(Qt::transparent);
            // Shrink each hidden glyph's advance to 1% of itself; a fixed
            // negative spacing would pull long runs like a link's URL back
            // over the visible text after them.
            f.setFontLetterSpacingType(QFont::PercentageSpacing);
            f.setFontLetterSpacing(1);
        }
        setFormat(start, count, f);
    }

    // Lay `bg` under `count` characters from `start`, keeping the rest of
    // each one's format: find's matches over the highlighting.
    void addBackground(int start, int count, quint32 bg) {
        for (int i = start; i < start + count; ++i) {
            QTextCharFormat f = format(i);
            f.setBackground(QColor::fromRgba(bg));
            setFormat(i, 1, f);
        }
    }

    // Add `px` after each of `count` characters from `start`, keeping the
    // rest of their format: room for a code span's box.
    void addSpacing(int start, int count, double px) {
        for (int i = start; i < start + count; ++i) {
            QTextCharFormat f = format(i);
            f.setFontLetterSpacingType(QFont::AbsoluteSpacing);
            f.setFontLetterSpacing(px);
            setFormat(i, 1, f);
        }
    }

    // Focus mode: the current line in `fg`, over the colours highlighting
    // gave it, as Typora greys every character away from the caret.
    // Hidden markers and struts stay transparent.
    void dimLine(quint32 fg) {
        const int n = currentBlock().length() - 1;
        for (int i = 0; i < n;) {
            const QTextCharFormat f = format(i);
            int j = i + 1;
            while (j < n && format(j) == f)
                ++j;
            if (!(f.hasProperty(QTextFormat::ForegroundBrush) && f.foreground().color().alpha() == 0)) {
                QTextCharFormat g = f;
                g.setForeground(QColor::fromRgba(fg));
                setFormat(i, j - i, g);
            }
            i = j;
        }
    }
    // Repaint line `line` alone (no such line: nothing).
    void rehighlightLine(int line) {
        const QTextBlock b = document() && line >= 0 ? document()->findBlockByNumber(line) : QTextBlock();
        if (b.isValid())
            rehighlightBlock(b);
    }

    // Make room for an inline object (src/strut.rs): the character at
    // `start` is set in strut font `step`, `pixelSize` high, spaced out to
    // `width`, and drawn in no colour.
    // With `keepAdvance` the character's own width in the text font is
    // added, for QML to draw it there again.
    void applyStrut(int start, int step, double pixelSize, double width, bool keepAdvance) {
        if (keepAdvance && document()) {
            const QString text = currentBlock().text();
            if (start >= 0 && start < text.size())
                width += QFontMetricsF(document()->defaultFont()).horizontalAdvance(text.at(start));
        }
        QTextCharFormat f;
        f.setFontFamilies({QStringLiteral("Typo Strut %1").arg(step)});
        f.setProperty(QTextFormat::FontPixelSize, qMax(1, qCeil(pixelSize)));
        f.setFontLetterSpacingType(QFont::AbsoluteSpacing);
        f.setFontLetterSpacing(width);
        f.setForeground(Qt::transparent);
        setFormat(start, 1, f);
    }

    // The x of document position `pos` and the y of its line's baseline,
    // in document coordinates (NaN when there is no such position). Lays
    // the text out first if it has changed.
    QPointF anchorAt(int pos) const {
        const QPointF none(qQNaN(), qQNaN());
        QTextDocument *doc = document();
        const QTextBlock b = doc ? doc->findBlock(pos) : QTextBlock();
        if (!b.isValid())
            return none;
        doc->documentLayout()->blockBoundingRect(b);
        const QTextLayout *layout = b.layout();
        const QTextLine line = layout ? layout->lineForTextPosition(pos - b.position()) : QTextLine();
        if (!line.isValid())
            return none;
        const qreal base = line.ascent() + (line.leadingIncluded() ? qMax(qreal(0), line.leading()) : 0);
        return layout->position() + QPointF(line.cursorToX(pos - b.position()), line.y() + base);
    }

protected:
    void highlightBlock(const QString &text) override { highlightText(text); }
    virtual void highlightText(const QString &text) = 0;
};

inline void typoSetDesktopFileName(const QString &name) { QGuiApplication::setDesktopFileName(name); }

inline bool typoHasRoots(const QQmlApplicationEngine &engine) { return !engine.rootObjects().isEmpty(); }

// File → Export → Image: premultiplied RGBA pixels as a PNG or JPEG file's
// bytes, by `format` ("png", "jpg"); nothing when Qt can't write them.
inline rust::Vec<std::uint8_t> typoEncodeImage(std::uint32_t width, std::uint32_t height, rust::Slice<const std::uint8_t> rgba,
                                               const QString &format)
{
    rust::Vec<std::uint8_t> out;
    if (rgba.size() < std::size_t(width) * height * 4)
        return out;
    const QImage image(rgba.data(), int(width), int(height), qsizetype(width) * 4, QImage::Format_RGBA8888_Premultiplied);
    QByteArray bytes;
    QBuffer buffer(&bytes);
    buffer.open(QIODevice::WriteOnly);
    const bool jpeg = format == QLatin1String("jpg");
    // JPEG has no alpha: flatten onto white, as the page is.
    QImage flat;
    if (jpeg) {
        flat = QImage(image.size(), QImage::Format_RGB32);
        flat.fill(Qt::white);
        QPainter(&flat).drawImage(0, 0, image);
    }
    if (!(jpeg ? flat : image).save(&buffer, jpeg ? "JPG" : "PNG", jpeg ? 92 : -1))
        return out;
    out.reserve(bytes.size());
    for (char c : bytes)
        out.push_back(std::uint8_t(c));
    return out;
}

// Defined in typo_math.cpp, which needs the bridge's Rust declarations.
void typoAddMathProvider(QQmlApplicationEngine &engine);
void typoAddDiagramProvider(QQmlApplicationEngine &engine);
void typoAddStrutFonts();
// Copy as Image: the formula `id` names (see src/qt/math.rs) on the clipboard.
bool typoCopyMathImage(const QString &id);

inline bool typoGrabWindow(const QString &path)
{
    for (QWindow *window : QGuiApplication::topLevelWindows())
        if (auto *quick = qobject_cast<QQuickWindow *>(window))
            return quick->grabWindow().save(path);
    return false;
}

// Test driver for `--script`: input goes through QtTest's window-level
// events, the same path real input takes, so shortcuts and key handlers
// behave as they do for the user.

inline QWindow *typoWindow()
{
    if (QWindow *w = QGuiApplication::focusWindow())
        return w;
    for (QWindow *w : QGuiApplication::topLevelWindows())
        if (qobject_cast<QQuickWindow *>(w))
            return w;
    return nullptr;
}

// Press and release a chord such as "Ctrl+S" or "Shift+Return".
inline bool typoSendKey(const QString &chord)
{
    QWindow *window = typoWindow();
    // Alt alone, as a hidden menu bar waits for.
    if (window && chord == QLatin1String("Alt")) {
        QTest::sendKeyEvent(QTest::Click, window, Qt::Key_Alt, QString(), Qt::NoModifier);
        return true;
    }
    const QKeySequence seq = QKeySequence::fromString(chord, QKeySequence::PortableText);
    if (!window || seq.isEmpty())
        return false;
    QKeyCombination k = seq[0];
    // Like the platform, deliver Shift+Tab as Backtab.
    if (k.key() == Qt::Key_Tab && (k.keyboardModifiers() & Qt::ShiftModifier))
        k = QKeyCombination(k.keyboardModifiers(), Qt::Key_Backtab);
    QString text;
    switch (k.key()) {
    case Qt::Key_Return: text = QStringLiteral("\r"); break;
    case Qt::Key_Tab: text = QStringLiteral("\t"); break;
    case Qt::Key_Space: text = QStringLiteral(" "); break;
    default: break;
    }
    if (k.keyboardModifiers() & (Qt::ControlModifier | Qt::AltModifier | Qt::MetaModifier))
        text.clear();
    QTest::sendKeyEvent(QTest::Click, window, k.key(), text, k.keyboardModifiers());
    return true;
}

// Type text one key at a time.
inline bool typoTypeText(const QString &text)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    for (const QChar c : text) {
        Qt::Key key = Qt::Key_unknown;
        Qt::KeyboardModifiers mods = Qt::NoModifier;
        // Qt's key codes for ASCII are the upper-case characters.
        if (c.unicode() < 128) {
            key = Qt::Key(c.toUpper().unicode());
            if (c.isUpper())
                mods = Qt::ShiftModifier;
        }
        QTest::sendKeyEvent(QTest::Click, window, key, QString(c), mods);
    }
    return true;
}

// Modifiers named in `mods`, such as "Ctrl+Shift".
inline Qt::KeyboardModifiers typoModifiers(const QString &mods)
{
    Qt::KeyboardModifiers m;
    if (mods.contains(QLatin1String("Shift")))
        m |= Qt::ShiftModifier;
    if (mods.contains(QLatin1String("Ctrl")))
        m |= Qt::ControlModifier;
    if (mods.contains(QLatin1String("Alt")))
        m |= Qt::AltModifier;
    return m;
}

// `mods` may also name the "Right" button.
inline bool typoClick(double x, double y, const QString &mods)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    const Qt::MouseButton button = mods.contains(QLatin1String("Right")) ? Qt::RightButton : Qt::LeftButton;
    QTest::mouseClick(window, button, typoModifiers(mods), QPointF(x, y).toPoint());
    return true;
}

inline bool typoHover(double x, double y)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    QTest::mouseMove(window, QPointF(x, y).toPoint());
    return true;
}

// Calls the window's `altTapped` when Alt goes down and up with nothing
// else pressed or clicked in between, as a menu bar that hides waits for.
// A key event reaches the filter once for each object it is sent to, so
// the tap is noted once, on the first release.
class TypoAltWatcher : public QObject
{
public:
    bool eventFilter(QObject *, QEvent *e) override
    {
        if (e->type() == QEvent::KeyPress) {
            const auto *k = static_cast<QKeyEvent *>(e);
            if (!k->isAutoRepeat())
                alone = k->key() == Qt::Key_Alt;
        } else if (e->type() == QEvent::KeyRelease) {
            const auto *k = static_cast<QKeyEvent *>(e);
            if (k->key() == Qt::Key_Alt && alone && !k->isAutoRepeat()) {
                alone = false;
                if (QWindow *w = typoWindow())
                    QMetaObject::invokeMethod(w, "altTapped", Qt::QueuedConnection);
            }
        } else if (e->type() == QEvent::MouseButtonPress || e->type() == QEvent::Wheel) {
            alone = false;
        }
        return false;
    }

private:
    bool alone = false;
};

inline void typoWatchAlt()
{
    static TypoAltWatcher *watcher = nullptr;
    if (!watcher) {
        watcher = new TypoAltWatcher;
        QGuiApplication::instance()->installEventFilter(watcher);
    }
}

// Turn the wheel at (x, y) by dy eighths of a degree.
inline bool typoWheel(double x, double y, double dy, const QString &mods)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    const QPointF at(x, y);
    QWheelEvent event(at, window->mapToGlobal(at), QPoint(), QPoint(0, int(dy)), Qt::NoButton, typoModifiers(mods),
                      Qt::NoScrollPhase, false);
    QGuiApplication::sendEvent(window, &event);
    return true;
}

// Drop files (paths one a line) at (x, y), as a file manager would.
inline bool typoDropFiles(double x, double y, const QString &paths)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    QMimeData data;
    QList<QUrl> urls;
    for (const QString &p : paths.split(QLatin1Char('\n'), Qt::SkipEmptyParts))
        urls << QUrl::fromLocalFile(p);
    data.setUrls(urls);
    const QPointF at(x, y);
    QDragEnterEvent enter(at.toPoint(), Qt::CopyAction, &data, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(window, &enter);
    QDragMoveEvent move(at.toPoint(), Qt::CopyAction, &data, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(window, &move);
    QDropEvent drop(at, Qt::CopyAction, &data, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(window, &drop);
    return drop.isAccepted();
}

// Files (paths one a line) dragged in from another program: over (x, y)
// (stage 0; the first enters the window), dropped there (1), or taken away
// again (2).
inline bool typoDragFiles(int stage, double x, double y, const QString &paths)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    static QMimeData data;
    static bool inside = false;
    const QPointF at(x, y);
    if (stage == 2) {
        inside = false;
        QDragLeaveEvent leave;
        QCoreApplication::sendEvent(window, &leave);
        return true;
    }
    if (!inside) {
        inside = true;
        QList<QUrl> urls;
        for (const QString &p : paths.split(QLatin1Char('\n'), Qt::SkipEmptyParts))
            urls << QUrl::fromLocalFile(p);
        data.setUrls(urls);
        QDragEnterEvent enter(at.toPoint(), Qt::CopyAction | Qt::MoveAction, &data, Qt::LeftButton, Qt::NoModifier);
        QCoreApplication::sendEvent(window, &enter);
    }
    QDragMoveEvent move(at.toPoint(), Qt::CopyAction | Qt::MoveAction, &data, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(window, &move);
    if (stage == 0)
        return move.isAccepted();
    inside = false;
    QDropEvent drop(at, Qt::CopyAction | Qt::MoveAction, &data, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(window, &drop);
    return drop.isAccepted();
}

// Press at (x1, y1), move to (x2, y2) in steps, release there.
inline bool typoDrag(double x1, double y1, double x2, double y2)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    QTest::mousePress(window, Qt::LeftButton, Qt::NoModifier, QPointF(x1, y1).toPoint());
    for (int i = 1; i <= 4; ++i) {
        const double t = i / 4.0;
        QTest::mouseMove(window, QPointF(x1 + (x2 - x1) * t, y1 + (y2 - y1) * t).toPoint());
    }
    QTest::mouseRelease(window, Qt::LeftButton, Qt::NoModifier, QPointF(x2, y2).toPoint());
    return true;
}

// One step of a drag, for tests: 0 presses the left button at a window
// point, 1 moves there (with the button, once pressed), 2 releases it.
inline bool typoMouse(int what, double x, double y)
{
    QWindow *window = typoWindow();
    if (!window)
        return false;
    const QPoint at = QPointF(x, y).toPoint();
    if (what == 0)
        QTest::mousePress(window, Qt::LeftButton, Qt::NoModifier, at);
    else if (what == 1)
        QTest::mouseMove(window, at);
    else
        QTest::mouseRelease(window, Qt::LeftButton, Qt::NoModifier, at);
    return true;
}

inline QString typoReadFile(const QString &path)
{
    QFile f(path);
    return f.open(QIODevice::ReadOnly) ? QString::fromUtf8(f.readAll()) : QString();
}

inline bool typoWriteFile(const QString &path, const QString &text)
{
    QFile f(path);
    return f.open(QIODevice::WriteOnly | QIODevice::Truncate) && f.write(text.toUtf8()) >= 0;
}

// The clipboard: plain text (the markdown source) plus HTML when given.
// typo's own copies also carry the selection's markdown, so pasting them
// back is exact rather than a round trip through HTML.
inline const QString typoMarkdownMime = QStringLiteral("application/x-typo-markdown");

inline void typoSetClipboard(const QString &text, const QString &html, const QString &markdown)
{
    auto *data = new QMimeData;
    data->setText(text);
    if (!html.isEmpty())
        data->setHtml(html);
    if (!markdown.isEmpty())
        data->setData(typoMarkdownMime, markdown.toUtf8());
    QGuiApplication::clipboard()->setMimeData(data);
}

inline QString typoClipboardMarkdown()
{
    const QMimeData *data = QGuiApplication::clipboard()->mimeData();
    return data && data->hasFormat(typoMarkdownMime) ? QString::fromUtf8(data->data(typoMarkdownMime)) : QString();
}

inline QString typoClipboardText()
{
    return QGuiApplication::clipboard()->text();
}

inline QString typoClipboardHtml()
{
    const QMimeData *data = QGuiApplication::clipboard()->mimeData();
    return data && data->hasHtml() ? data->html() : QString();
}

// Files copied in a file manager, as local paths, one a line.
inline QString typoClipboardFiles()
{
    const QMimeData *data = QGuiApplication::clipboard()->mimeData();
    QStringList paths;
    if (data && data->hasUrls()) {
        for (const QUrl &url : data->urls()) {
            if (url.isLocalFile())
                paths << url.toLocalFile();
        }
    }
    return paths.join(QLatin1Char('\n'));
}

// A picture on the clipboard (a screenshot, say) saved in `folder` as
// image-<local time to the millisecond>.png, as Typora names them. The
// path, or "" when there is no picture or it can't be written.
inline QString typoSaveClipboardImage(const QString &folder)
{
    const QMimeData *data = QGuiApplication::clipboard()->mimeData();
    if (!data || !data->hasImage() || !QDir().mkpath(folder))
        return QString();
    const QString stamp = QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMddHHmmsszzz"));
    const QString path = QDir(folder).filePath(QStringLiteral("image-%1.png").arg(stamp));
    // PNG data is kept as it came; anything else is converted.
    if (data->hasFormat(QStringLiteral("image/png"))) {
        QFile f(path);
        if (f.open(QIODevice::WriteOnly) && f.write(data->data(QStringLiteral("image/png"))) > 0)
            return path;
    }
    const QImage image = qvariant_cast<QImage>(data->imageData());
    return !image.isNull() && image.save(path, "PNG") ? path : QString();
}

// For the test driver: files as a file manager copies them, and a picture.
inline void typoSetClipboardFiles(const QString &paths)
{
    auto *data = new QMimeData;
    QList<QUrl> urls;
    for (const QString &p : paths.split(QLatin1Char('\n'), Qt::SkipEmptyParts))
        urls << QUrl::fromLocalFile(p);
    data->setUrls(urls);
    QGuiApplication::clipboard()->setMimeData(data);
}

inline bool typoSetClipboardImage(const QString &path)
{
    const QImage image(path);
    if (image.isNull())
        return false;
    QGuiApplication::clipboard()->setImage(image);
    return true;
}

// Move a file to the trash, as Delete Image File does.
inline bool typoMoveToTrash(const QString &path)
{
    return QFile::moveToTrash(path);
}

// The text a reader sees in `html`, for Copy as Plain Text.
inline QString typoHtmlToText(const QString &html)
{
    return QTextDocumentFragment::fromHtml(html).toPlainText();
}
