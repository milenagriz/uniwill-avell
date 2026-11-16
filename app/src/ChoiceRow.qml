import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// A few exclusive choices side by side; the one in force is filled with the
// accent.
RowLayout {
    id: choice
    property var options: []    // [{ value, label }]
    property var current
    signal chosen(var value)
    spacing: 6

    Repeater {
        model: choice.options
        Rectangle {
            id: option
            required property var modelData
            readonly property bool current: modelData.value === choice.current
            implicitWidth: label.implicitWidth + 28
            implicitHeight: 36
            radius: theme.radius
            color: current ? theme.accent : hover.hovered ? theme.border : "transparent"
            border.width: 1
            border.color: activeFocus ? theme.foreground : current ? theme.accent : theme.border
            activeFocusOnTab: true
            Keys.onSpacePressed: choice.chosen(modelData.value)
            Keys.onReturnPressed: choice.chosen(modelData.value)

            Txt {
                id: label
                anchors.centerIn: parent
                text: option.modelData.label
                color: option.current ? theme.accentForeground : theme.foreground
            }
            HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
            TapHandler { onTapped: choice.chosen(option.modelData.value) }
        }
    }
}
