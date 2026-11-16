import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// One setting: its name and what it does on the left, its control on the
// right. A feature this machine's driver does not expose stays visible, with
// its control disabled and a note saying why.
RowLayout {
    id: row
    property string label
    property string hint
    property bool unavailable: false
    default property alias control: slot.data

    Layout.fillWidth: true
    spacing: 16

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Txt {
            text: row.label
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
        Txt {
            text: row.hint
            visible: text !== ""
            color: theme.muted
            font.pixelSize: 12
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
        Txt {
            text: qsTr("Unavailable: not exposed by the driver on this machine")
            visible: row.unavailable
            color: theme.yellow
            font.pixelSize: 12
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
    }
    RowLayout {
        id: slot
        enabled: !row.unavailable
        opacity: enabled ? 1 : 0.45
        spacing: 8
    }
}
