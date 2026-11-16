import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// The machine's state, as `avellctl status` prints it.
ScrollPage {
    id: page
    property var app
    heading: qsTr("Status")

    function reading(value, unit) {
        if (!backend.running)
            return "—"
        return app.has(value) ? value + unit : qsTr("unavailable")
    }
    function fanControl() {
        var control = app.st("fans.control")
        if (!app.has(control))
            return reading(control, "")
        var target = app.st("fans.target_percent")
        return app.fanControlLabel(control) + (app.has(target) ? "  " + target + " %" : "")
    }
    function gpuPower() {
        var draw = app.st("gpu.power_draw_w"), limit = app.st("gpu.power_limit_w")
        if (!app.has(draw) || !app.has(limit))
            return reading(undefined, "")
        return qsTr("%1 W of %2 W").arg(Number(draw).toLocaleString(Qt.locale(), "f", 1)).arg(Number(limit).toLocaleString(Qt.locale(), "f", 0))
    }
    function powerSource() {
        var onAc = app.st("battery.on_ac")
        return app.has(onAc) ? (onAc ? qsTr("Charger") : qsTr("Battery")) : reading(onAc, "")
    }

    GridLayout {
        Layout.fillWidth: true
        columns: page.width > 760 ? 2 : 1
        columnSpacing: 16
        rowSpacing: 16

        Card {
            title: qsTr("Power profile")
            Layout.fillHeight: true
            StatLine { label: qsTr("Active"); value: app.has(app.st("profile")) ? app.profileLabel(app.st("profile")) : page.reading(undefined, "") }
            StatLine { label: qsTr("Hold"); value: app.has(app.st("hold")) ? app.st("hold") : (backend.running ? qsTr("none") : "—") }
            StatLine { label: qsTr("Power source"); value: page.powerSource() }
        }
        Card {
            title: qsTr("Temperatures")
            Layout.fillHeight: true
            StatLine { label: qsTr("CPU"); value: page.reading(app.st("temperatures.cpu_c"), " °C") }
            StatLine { label: qsTr("Graphics card"); value: page.reading(app.st("temperatures.gpu_c"), " °C") }
        }
        Card {
            title: qsTr("Fans")
            Layout.fillHeight: true
            StatLine { label: qsTr("Control"); value: page.fanControl() }
            StatLine { label: qsTr("Driver mode"); value: page.reading(app.st("fans.mode"), "") }
            Repeater {
                model: app.st("fans.fans") || []
                StatLine {
                    required property var modelData
                    required property int index
                    label: qsTr("Fan %1").arg(index + 1)
                    value: qsTr("%1 rpm (%2/255)").arg(modelData.rpm).arg(modelData.duty)
                }
            }
        }
        Card {
            title: qsTr("Graphics card")
            Layout.fillHeight: true
            StatLine { label: qsTr("State"); value: page.reading(app.st("gpu.state"), "") }
            StatLine { label: qsTr("TGP offset"); value: page.reading(app.st("gpu.tgp_offset_w"), " W") }
            StatLine { label: qsTr("Power"); value: page.gpuPower() }
            StatLine { label: qsTr("Load"); value: page.reading(app.st("gpu.utilization_percent"), " %") }
        }
        Card {
            title: qsTr("CPU power limits")
            Layout.fillHeight: true
            StatLine { label: qsTr("Long-term"); value: page.reading(app.st("cpu.long_term_w"), " W") }
            StatLine { label: qsTr("Short-term"); value: page.reading(app.st("cpu.short_term_w"), " W") }
        }
        Card {
            title: qsTr("Battery")
            Layout.fillHeight: true
            StatLine { label: qsTr("Charge"); value: page.reading(app.st("battery.capacity_percent"), " %") }
            StatLine { label: qsTr("State"); value: page.reading(app.st("battery.state"), "") }
            StatLine { label: qsTr("Cycles"); value: page.reading(app.st("battery.cycle_count"), "") }
            StatLine { label: qsTr("Health"); value: page.reading(app.st("battery.health_percent"), " %") }
            StatLine { label: qsTr("Charging profile"); value: app.has(app.st("battery.charging_profile")) ? app.chargingProfileLabel(app.st("battery.charging_profile")) : page.reading(undefined, "") }
            StatLine { label: qsTr("Charge cap"); value: page.reading(app.st("battery.charge_cap_percent"), " %") }
        }
        Card {
            title: qsTr("Keyboard backlight")
            Layout.fillHeight: true
            StatLine {
                label: qsTr("Brightness")
                value: app.has(app.st("backlight.brightness"))
                       ? qsTr("%1 of %2").arg(app.st("backlight.brightness")).arg(page.reading(app.st("backlight.max_brightness"), ""))
                       : page.reading(undefined, "")
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Txt { text: qsTr("Colour"); color: theme.muted; Layout.preferredWidth: 130 }
                Rectangle {
                    visible: app.keyboardColor() !== ""
                    width: 16
                    height: 16
                    radius: Math.min(4, theme.radius)
                    color: visible ? app.keyboardColor() : "transparent"
                    border.color: theme.border
                }
                Txt { text: app.keyboardColor() !== "" ? app.keyboardColor() : page.reading(undefined, ""); Layout.fillWidth: true }
            }
        }
        Card {
            title: qsTr("Unavailable")
            Layout.fillHeight: true
            visible: (app.st("unavailable") || []).length > 0
            Txt {
                text: (app.st("unavailable") || []).join(", ")
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
            Txt {
                text: qsTr("Not exposed by the driver on this machine.")
                color: theme.muted
                font.pixelSize: 12
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
        }
    }
}
