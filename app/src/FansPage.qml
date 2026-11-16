import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// Who runs the fans now, and each Power profile's fan curve.
ScrollPage {
    id: page
    property var app
    // The curve on show: the one picked, else the active Power profile's.
    property string picked: ""
    readonly property string shown: picked !== "" ? picked : app.profileNames.indexOf(app.st("profile")) >= 0 ? app.st("profile") : "balanced"
    heading: qsTr("Fans")

    readonly property bool noControl: app.unavailable("fan_control")
    readonly property var hottest: {
        var cpu = app.st("temperatures.cpu_c"), gpu = app.st("temperatures.gpu_c")
        if (!app.has(cpu) && !app.has(gpu))
            return undefined
        return Math.max(app.has(cpu) ? cpu : 0, app.has(gpu) ? gpu : 0)
    }

    Card {
        title: qsTr("Fan control")
        SettingRow {
            label: qsTr("Who runs the fans")
            hint: qsTr("Curve: the daemon, along the active Power profile's fan curve. Fixed: one speed. Max fan: both fans flat out. Automatic: the firmware. Lasts until the next change or a restart of the daemon.")
            unavailable: page.noControl
            ChoiceRow {
                options: [
                    { value: "curve", label: qsTr("Curve") },
                    { value: "fixed", label: qsTr("Fixed") },
                    { value: "max", label: qsTr("Max fan") },
                    { value: "automatic", label: qsTr("Automatic") }
                ]
                current: app.st("fans.control")
                onChosen: (value) => backend.setFan(value, Math.round(fixed.value))
            }
        }
        SettingRow {
            label: qsTr("Fixed speed")
            hint: qsTr("Moving it hands the fans to this speed.")
            unavailable: page.noControl
            LiveSlider {
                id: fixed
                from: 0
                to: 100
                machine: app.has(app.st("fans.target_percent")) ? app.st("fans.target_percent") : 50
                onCommitted: (value) => backend.setFan("fixed", value)
            }
            Txt { text: Math.round(fixed.value) + " %"; Layout.preferredWidth: 48; horizontalAlignment: Text.AlignRight }
        }
    }

    Card {
        title: qsTr("Fan curves")
        Txt {
            text: qsTr("The fan speed for the hotter of the CPU and the graphics card, one curve per Power profile. Temperatures have to rise from point to point and speeds must never fall; the daemon checks the curve when you save.")
            color: theme.muted
            font.pixelSize: 12
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
        ChoiceRow {
            options: app.profileNames.map(name => ({
                value: name,
                label: name === app.st("profile") ? qsTr("%1 (active)").arg(app.profileLabel(name)) : app.profileLabel(name)
            }))
            current: page.shown
            onChosen: (value) => { page.picked = value; curve.selected = -1 }
        }
        CurveEditor {
            id: curve
            Layout.fillWidth: true
            Layout.preferredHeight: 380
            points: app.draft.profiles[page.shown].fan_curve
            markerTemp: page.shown === app.st("profile") ? page.hottest : undefined
            markerPercent: page.shown === app.st("profile") && app.has(app.st("fans.target_percent")) ? app.st("fans.target_percent") : undefined
            onEdited: (points) => app.edit(draft => draft.profiles[page.shown].fan_curve = points)
        }
    }

    Card {
        title: qsTr("Behaviour")
        SettingRow {
            label: qsTr("When the daemon starts")
            hint: qsTr("Who runs the fans from the start: the daemon along the curve, or the firmware.")
            ChoiceRow {
                options: [
                    { value: "curve", label: qsTr("Curve") },
                    { value: "automatic", label: qsTr("Automatic") }
                ]
                current: app.draft.fans.mode
                onChosen: (value) => app.edit(draft => draft.fans.mode = value)
            }
        }
        SettingRow {
            label: qsTr("Hysteresis")
            hint: qsTr("The fans slow down only once it is this much cooler than where they last sped up.")
            SpinBox {
                from: 0
                to: 20
                editable: true
                value: app.draft.fans.hysteresis_c
                onValueModified: app.edit(draft => draft.fans.hysteresis_c = value)
            }
            Txt { text: qsTr("°C"); color: theme.muted }
        }
    }
}
