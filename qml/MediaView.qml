// A lone `<video>` or `<audio>`, playing, as Chromium's player in Typora:
// a bar of play, time and progress, always under audio and over video
// while the pointer is on it. A video stands centred at its own size, or
// its tag's, never wider than the page; until it loads, 300 by 150.
// Clicks reach the bar through the view (`controlAt`).
import QtQuick
import QtMultimedia
import Typo

Item {
    id: media
    // Editor.mediaOf's: {kind, src, width, height, autoplay, loop, muted}.
    property var info
    property url url
    property real room
    property real scale: 1
    property real em: 16
    readonly property bool audio: info.kind === "audio"
    readonly property bool failed: player.error !== MediaPlayer.NoError
    readonly property bool playing: player.playbackState === MediaPlayer.PlayingState
    readonly property alias player: player

    readonly property size fitted: {
        const r = video.sourceRect
        return Ui.imageSize(r.width > 0 ? r.width : 300, r.width > 0 ? r.height : 150, [info.width, info.height, ""], room, scale)
    }
    width: audio ? Math.min(room, 300 * scale) : fitted.width
    height: audio ? 54 * scale : fitted.height

    MediaPlayer {
        id: player
        source: media.url
        audioOutput: AudioOutput { muted: media.info.muted }
        videoOutput: media.audio ? null : video
        loops: media.info.loop ? MediaPlayer.Infinite : 1
        onMediaStatusChanged: {
            if (mediaStatus !== MediaPlayer.LoadedMedia)
                return
            if (media.info.autoplay)
                play()
            else if (!media.audio)
                pause() // which shows the first frame
        }
    }
    VideoOutput {
        id: video
        visible: !media.audio
        anchors.fill: parent
    }
    HoverHandler {
        id: hover
    }

    function clock(ms) {
        const s = Math.floor(Math.max(0, ms) / 1000)
        return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0")
    }

    Rectangle {
        id: bar
        readonly property color ink: media.audio ? Theme.foreground : "white"
        visible: media.audio || hover.hovered
        anchors.bottom: parent.bottom
        width: parent.width
        height: Math.min(parent.height, (media.audio ? 54 : 40) * media.scale)
        radius: media.audio ? height / 2 : 0
        color: media.audio ? Theme.codeBackground : Qt.rgba(0, 0, 0, 0.45)
        Glyph {
            id: toggle
            x: 14 * media.scale
            anchors.verticalCenter: parent.verticalCenter
            name: media.playing ? "pause" : "play"
            size: media.em
            color: bar.ink
        }
        Text {
            id: time
            x: toggle.x + toggle.width + 10 * media.scale
            anchors.verticalCenter: parent.verticalCenter
            text: media.clock(player.position) + " / " + media.clock(player.duration)
            color: bar.ink
            font.family: Ui.textFont
            font.pixelSize: media.em * 0.8
        }
        Rectangle {
            id: track
            x: time.x + time.width + 12 * media.scale
            anchors.verticalCenter: parent.verticalCenter
            width: Math.max(0, bar.width - x - 18 * media.scale)
            height: 4 * media.scale
            radius: height / 2
            color: Qt.alpha(bar.ink, 0.3)
            Rectangle {
                width: player.duration > 0 ? parent.width * player.position / player.duration : 0
                height: parent.height
                radius: parent.radius
                color: bar.ink
            }
        }
    }

    // The track seeks to where it is clicked, the rest of the bar plays or
    // pauses; the picture itself is the paragraph's, for the caret.
    function controlAt(x, y) {
        if (!bar.visible || y < bar.y || x < 0 || x > width)
            return null
        if (x >= track.x - 6 * scale)
            return { activate: () => player.setPosition(player.duration * Math.max(0, Math.min(1, (x - track.x) / track.width))) }
        return { activate: () => media.playing ? player.pause() : player.play() }
    }
}
