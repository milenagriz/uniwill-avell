import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// Battery care: the charging profile and the charge cap. Both apply at once.
ScrollPage {
    id: page
    property var app
    heading: qsTr("Battery")

    function reading(value, unit) {
        if (!backend.running)
            return "—"
        return app.has(value) ? value + unit : qsTr("unavailable")
    }

    Card {
        title: qsTr("Battery")
        StatLine { label: qsTr("Charge"); value: page.reading(app.st("battery.capacity_percent"), " %") }
        StatLine { label: qsTr("State"); value: page.reading(app.st("battery.state"), "") }
        StatLine { label: qsTr("Cycles"); value: page.reading(app.st("battery.cycle_count"), "") }
        StatLine { label: qsTr("Health"); value: page.reading(app.st("battery.health_percent"), " %") }
    }

    Card {
        title: qsTr("Battery care")
        SettingRow {
            label: qsTr("Charging profile")
            hint: qsTr("The firmware's way of charging: high capacity for full runtime, balanced, or stationary for a machine that stays plugged in.")
            unavailable: app.unavailable("charging_profile")
            ChoiceRow {
                options: ["high-capacity", "balanced", "stationary"].map(name => ({ value: name, label: app.chargingProfileLabel(name) }))
                current: app.st("battery.charging_profile")
                onChosen: (value) => backend.setChargingProfile(value)
            }
        }
        SettingRow {
            label: qsTr("Charge cap")
            hint: qsTr("Charging stops at this level.")
            unavailable: app.unavailable("charge_cap")
            LiveSlider {
                id: cap
                from: 20
                to: 100
                machine: app.has(app.st("battery.charge_cap_percent")) ? app.st("battery.charge_cap_percent") : 100
                onCommitted: (value) => backend.setChargeCap(value)
            }
            Txt { text: Math.round(cap.value) + " %"; Layout.preferredWidth: 48; horizontalAlignment: Text.AlignRight }
        }
    }
}
