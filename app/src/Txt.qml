import QtQuick

// Every text in the app: plain text always, since much of it comes from the
// daemon, in the theme's colour.
Text {
    color: theme.foreground
    textFormat: Text.PlainText
    font.pixelSize: 14
}
