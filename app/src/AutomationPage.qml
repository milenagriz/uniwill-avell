import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// The charger rules and game detection.
ScrollPage {
    id: page
    property var app
    heading: qsTr("Automation")

    readonly property var charger: app.draft.charger
    readonly property var game: app.draft.game_detection

    Card {
        title: qsTr("Charger rules")
        SettingRow {
            label: qsTr("Change the Power profile with the power source")
            hint: qsTr("Off: plugging and unplugging change nothing. On battery the TGP offset is 0 either way.")
            Switch {
                checked: page.charger.enabled
                onToggled: app.edit(draft => draft.charger.enabled = checked)
            }
        }
        SettingRow {
            label: qsTr("On the charger")
            hint: qsTr("A manual change of Power profile while plugged in replaces this.")
            ComboBox {
                enabled: page.charger.enabled
                Layout.preferredWidth: 200
                model: app.profileNames.map(name => app.profileLabel(name))
                currentIndex: app.profileNames.indexOf(page.charger.on_ac)
                onActivated: (index) => app.edit(draft => draft.charger.on_ac = app.profileNames[index])
            }
        }
        SettingRow {
            label: qsTr("On battery")
            hint: qsTr("A manual change of Power profile while unplugged replaces this.")
            ComboBox {
                enabled: page.charger.enabled
                Layout.preferredWidth: 200
                model: app.profileNames.map(name => app.profileLabel(name))
                currentIndex: app.profileNames.indexOf(page.charger.on_battery)
                onActivated: (index) => app.edit(draft => draft.charger.on_battery = app.profileNames[index])
            }
        }
    }

    Card {
        title: qsTr("Game detection")
        SettingRow {
            label: qsTr("Hold the performance Power profile while the graphics card is busy")
            hint: qsTr("Nothing is held on battery. Choosing another Power profile during a hold cancels it.")
            Switch {
                checked: page.game.enabled
                onToggled: app.edit(draft => draft.game_detection.enabled = checked)
            }
        }
        SettingRow {
            label: qsTr("Busy from")
            hint: qsTr("The graphics card counts as busy at this load or above.")
            SpinBox {
                enabled: page.game.enabled
                from: 1
                to: 100
                editable: true
                value: page.game.load_percent
                onValueModified: app.edit(draft => draft.game_detection.load_percent = value)
            }
            Txt { text: qsTr("%"); color: theme.muted }
        }
        SettingRow {
            label: qsTr("Hold after")
            hint: qsTr("Busy this long: the hold starts.")
            SpinBox {
                enabled: page.game.enabled
                from: 0
                to: 3600
                editable: true
                value: page.game.hold_after_s
                onValueModified: app.edit(draft => draft.game_detection.hold_after_s = value)
            }
            Txt { text: qsTr("s"); color: theme.muted }
        }
        SettingRow {
            label: qsTr("Release after")
            hint: qsTr("Below the threshold this long: the hold is released.")
            SpinBox {
                enabled: page.game.enabled
                from: 0
                to: 3600
                editable: true
                value: page.game.release_after_s
                onValueModified: app.edit(draft => draft.game_detection.release_after_s = value)
            }
            Txt { text: qsTr("s"); color: theme.muted }
        }
        Txt {
            visible: app.has(app.st("hold"))
            text: qsTr("Holding now: %1").arg(app.has(app.st("hold")) ? app.st("hold") : "")
            color: theme.yellow
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
    }
}
