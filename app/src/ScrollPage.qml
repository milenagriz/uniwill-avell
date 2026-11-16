import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// One page of the window: a heading over a scrolling column of cards.
Flickable {
    id: page
    property string heading
    default property alias content: column.data

    contentWidth: width
    contentHeight: column.implicitHeight + 48
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}

    ColumnLayout {
        id: column
        x: 24
        y: 24
        width: Math.min(page.width - 48, 920)
        spacing: 16

        Txt {
            text: page.heading
            font.pixelSize: 22
            font.weight: Font.DemiBold
        }
    }
}
