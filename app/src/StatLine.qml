import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// One reading: what it is, and its value.
RowLayout {
    property alias label: name.text
    property alias value: reading.text
    Layout.fillWidth: true
    spacing: 12

    Txt {
        id: name
        color: theme.muted
        Layout.preferredWidth: 130
        Layout.alignment: Qt.AlignTop
    }
    Txt {
        id: reading
        Layout.fillWidth: true
        wrapMode: Text.Wrap
    }
}
