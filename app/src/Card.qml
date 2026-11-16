import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// A titled panel that groups related settings.
Rectangle {
    id: card
    property string title
    default property alias content: body.data

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight + 32
    color: theme.surface
    border.color: theme.border
    border.width: 1
    radius: theme.radius

    ColumnLayout {
        id: column
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Txt {
            text: card.title
            visible: text !== ""
            font.pixelSize: 16
            font.weight: Font.DemiBold
        }
        ColumnLayout {
            id: body
            Layout.fillWidth: true
            spacing: 10
        }
        Item { Layout.fillHeight: true }
    }
}
