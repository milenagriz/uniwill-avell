import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// A button in the theme's colours; primary fills with the accent.
Rectangle {
    id: button
    property alias text: label.text
    property bool primary: false
    signal clicked()

    implicitWidth: label.implicitWidth + 32
    implicitHeight: 36
    radius: theme.radius
    opacity: enabled ? 1 : 0.45
    color: primary ? theme.accent : hover.hovered ? theme.border : "transparent"
    border.width: 1
    border.color: activeFocus ? theme.foreground : primary ? theme.accent : theme.border
    activeFocusOnTab: true
    Keys.onSpacePressed: button.clicked()
    Keys.onReturnPressed: button.clicked()

    Txt {
        id: label
        anchors.centerIn: parent
        color: button.primary ? theme.accentForeground : theme.foreground
    }
    HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: button.clicked() }
}
