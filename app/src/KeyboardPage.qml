import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// The keyboard backlight: brightness and colour. Both apply at once.
ScrollPage {
    id: page
    property var app
    heading: qsTr("Keyboard")

    readonly property bool noBacklight: app.unavailable("keyboard_backlight")
    readonly property var color: app.st("backlight.color")
    readonly property bool hasColor: app.has(color) && color.length === 3
    readonly property bool singleColour: backend.running && !noBacklight && !hasColor

    function channel(index) { return hasColor ? color[index] : 0 }
    function send() { backend.setKeyboardColor(Math.round(red.value), Math.round(green.value), Math.round(blue.value)) }

    Card {
        title: qsTr("Keyboard backlight")
        SettingRow {
            label: qsTr("Brightness")
            hint: qsTr("0 is off.")
            unavailable: page.noBacklight
            LiveSlider {
                id: brightness
                from: 0
                to: app.has(app.st("backlight.max_brightness")) ? app.st("backlight.max_brightness") : 1
                machine: app.has(app.st("backlight.brightness")) ? app.st("backlight.brightness") : 0
                onCommitted: (value) => backend.setKeyboardBrightness(value)
            }
            Txt { text: Math.round(brightness.value); Layout.preferredWidth: 48; horizontalAlignment: Text.AlignRight }
        }
    }

    Card {
        title: qsTr("Colour")
        Txt {
            visible: page.singleColour
            text: qsTr("Unavailable: this keyboard has one colour.")
            color: theme.yellow
            font.pixelSize: 12
        }
        SettingRow {
            label: qsTr("Now")
            unavailable: page.noBacklight
            Rectangle {
                width: 96
                height: 28
                radius: theme.radius
                color: app.keyboardColor() !== "" ? app.keyboardColor() : "transparent"
                border.color: theme.border
            }
            Txt { text: app.keyboardColor(); Layout.preferredWidth: 70 }
        }
        SettingRow {
            label: qsTr("Presets")
            unavailable: page.noBacklight
            enabled: !page.singleColour
            Repeater {
                model: ["#ffffff", "#ff0000", "#ff8000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff"]
                Rectangle {
                    id: swatch
                    required property string modelData
                    width: 28
                    height: 28
                    radius: theme.radius
                    color: modelData
                    border.color: theme.border
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            var c = Qt.color(swatch.modelData)
                            backend.setKeyboardColor(Math.round(c.r * 255), Math.round(c.g * 255), Math.round(c.b * 255))
                        }
                    }
                }
            }
        }
        SettingRow {
            label: qsTr("Red")
            unavailable: page.noBacklight
            enabled: !page.singleColour
            LiveSlider { id: red; from: 0; to: 255; machine: page.channel(0); onCommitted: page.send() }
            Txt { text: Math.round(red.value); Layout.preferredWidth: 48; horizontalAlignment: Text.AlignRight }
        }
        SettingRow {
            label: qsTr("Green")
            unavailable: page.noBacklight
            enabled: !page.singleColour
            LiveSlider { id: green; from: 0; to: 255; machine: page.channel(1); onCommitted: page.send() }
            Txt { text: Math.round(green.value); Layout.preferredWidth: 48; horizontalAlignment: Text.AlignRight }
        }
        SettingRow {
            label: qsTr("Blue")
            unavailable: page.noBacklight
            enabled: !page.singleColour
            LiveSlider { id: blue; from: 0; to: 255; machine: page.channel(2); onCommitted: page.send() }
            Txt { text: Math.round(blue.value); Layout.preferredWidth: 48; horizontalAlignment: Text.AlignRight }
        }
    }
}
