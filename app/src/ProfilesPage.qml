import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// The active Power profile, and the bundle each one applies.
ScrollPage {
    id: page
    property var app
    heading: qsTr("Power profiles")

    Card {
        title: qsTr("Active Power profile")
        SettingRow {
            label: qsTr("Power profile")
            hint: qsTr("Set through power-profiles-daemon, like the power menu and the Fn key. With the charger rules on, it becomes the Power profile of the power source in use.")
            ChoiceRow {
                options: app.profileNames.map(name => ({ value: name, label: app.profileLabel(name) }))
                current: app.st("profile")
                onChosen: (value) => backend.setProfile(value)
            }
        }
        Txt {
            visible: app.has(app.st("hold"))
            text: qsTr("Held: %1").arg(app.has(app.st("hold")) ? app.st("hold") : "")
            color: theme.yellow
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
    }

    Repeater {
        model: app.profileNames
        Card {
            id: bundle
            required property string modelData
            readonly property var values: app.draft.profiles[modelData]
            title: modelData === app.st("profile") ? qsTr("%1 (active)").arg(app.profileLabel(modelData)) : app.profileLabel(modelData)

            SettingRow {
                label: qsTr("TGP offset")
                hint: qsTr("Extra watts for the graphics card on top of its base power budget. On battery it is always 0.")
                unavailable: app.unavailable("tgp_offset")
                SpinBox {
                    from: 0
                    to: 20
                    editable: true
                    value: bundle.values.tgp_offset_w
                    onValueModified: app.edit(draft => draft.profiles[bundle.modelData].tgp_offset_w = value)
                }
                Txt { text: qsTr("W"); color: theme.muted }
            }
            SettingRow {
                label: qsTr("Dynamic Boost")
                hint: qsTr("Moves spare power budget from the processor to the graphics card while both are under load.")
                Switch {
                    checked: bundle.values.dynamic_boost
                    onToggled: app.edit(draft => draft.profiles[bundle.modelData].dynamic_boost = checked)
                }
            }
            SettingRow {
                label: qsTr("CPU long-term power limit")
                hint: qsTr("What the processor may draw over time.")
                unavailable: app.unavailable("cpu_power_limits")
                SpinBox {
                    from: 5
                    to: 120
                    editable: true
                    value: bundle.values.cpu_long_term_w
                    onValueModified: app.edit(draft => draft.profiles[bundle.modelData].cpu_long_term_w = value)
                }
                Txt { text: qsTr("W"); color: theme.muted }
            }
            SettingRow {
                label: qsTr("CPU short-term power limit")
                hint: qsTr("What it may draw in short bursts. Not below the long-term limit.")
                unavailable: app.unavailable("cpu_power_limits")
                SpinBox {
                    from: 5
                    to: 120
                    editable: true
                    value: bundle.values.cpu_short_term_w
                    onValueModified: app.edit(draft => draft.profiles[bundle.modelData].cpu_short_term_w = value)
                }
                Txt { text: qsTr("W"); color: theme.muted }
            }
        }
    }
}
