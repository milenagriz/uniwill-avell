import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

ApplicationWindow {
    id: win
    width: 1080
    height: 760
    minimumWidth: 720
    minimumHeight: 480
    visible: true
    title: qsTr("Avell Control")
    color: theme.background

    Material.theme: theme.dark ? Material.Dark : Material.Light
    Material.accent: theme.accent
    Material.primary: theme.accent
    Material.background: theme.surface
    Material.foreground: theme.foreground
    Material.roundedScale: theme.radius

    // The configuration being edited: a copy of the daemon's, which follows
    // it until the owner changes something and again once that is saved.
    property var draft: ({})
    property bool hasConfig: false
    property bool dirty: false
    // The daemon's configuration changed (the CLI, a manual change of Power
    // profile) while there were unsaved edits here.
    property bool outdated: false
    property bool saving: false
    property string saveError: ""

    property int page: 0
    property bool helpVisible: false

    readonly property var profileNames: ["power-saver", "balanced", "performance"]
    readonly property var pages: [qsTr("Status"), qsTr("Fans"), qsTr("Power profiles"), qsTr("Automation"), qsTr("Battery"), qsTr("Keyboard")]

    function profileLabel(name) {
        return name === "power-saver" ? qsTr("Power saver") : name === "balanced" ? qsTr("Balanced")
             : name === "performance" ? qsTr("Performance") : name
    }
    function chargingProfileLabel(name) {
        return name === "high-capacity" ? qsTr("High capacity") : name === "balanced" ? qsTr("Balanced")
             : name === "stationary" ? qsTr("Stationary") : name
    }
    function fanControlLabel(name) {
        return name === "curve" ? qsTr("Curve") : name === "fixed" ? qsTr("Fixed") : name === "max" ? qsTr("Max fan")
             : name === "automatic" ? qsTr("Automatic") : name
    }

    function has(value) { return value !== undefined && value !== null }
    // A value of the status by its path ("fans.control"); undefined when the
    // daemon is stopped or the value is not there.
    function st(path) {
        var value = backend.status
        var keys = path.split(".")
        for (var i = 0; i < keys.length; i++) {
            if (!has(value))
                return undefined
            value = value[keys[i]]
        }
        return value
    }
    function unavailable(feature) { return (st("unavailable") || []).indexOf(feature) >= 0 }
    function keyboardColor() {
        var c = st("backlight.color")
        if (!has(c) || c.length !== 3)
            return ""
        var hex = (n) => ("0" + Math.max(0, Math.min(255, n)).toString(16)).slice(-2)
        return "#" + hex(c[0]) + hex(c[1]) + hex(c[2])
    }

    function adopt() {
        if (backend.config.profiles === undefined)
            return
        draft = JSON.parse(JSON.stringify(backend.config))
        hasConfig = true
        dirty = false
        outdated = false
        saveError = ""
    }
    function edit(change) {
        var next = JSON.parse(JSON.stringify(draft))
        change(next)
        draft = next
        dirty = true
    }
    function save() {
        if (!dirty || saving || !backend.running)
            return
        saving = true
        backend.saveConfig(JSON.stringify(draft))
    }
    function quit() {
        if (dirty)
            discard.open()
        else
            Qt.quit()
    }

    Component.onCompleted: adopt()
    Connections {
        target: backend
        function onConfigChanged() {
            if (win.dirty)
                win.outdated = true
            else
                win.adopt()
        }
        function onConfigSaved() {
            win.saving = false
            win.dirty = false
            win.adopt()
        }
        // The edit stays, so the owner can fix it and save again.
        function onConfigRejected(reason) {
            win.saving = false
            win.saveError = reason
        }
    }

    onClosing: (close) => {
        if (win.dirty) {
            close.accepted = false
            discard.open()
        }
    }

    Shortcut { sequence: "?"; context: Qt.ApplicationShortcut; onActivated: win.helpVisible = !win.helpVisible }
    Shortcut { sequence: "Q"; context: Qt.ApplicationShortcut; onActivated: win.quit() }
    Shortcut { sequence: "Escape"; context: Qt.ApplicationShortcut; enabled: win.helpVisible; onActivated: win.helpVisible = false }
    Shortcut { sequences: [StandardKey.Save]; context: Qt.ApplicationShortcut; onActivated: win.save() }
    Shortcut { sequence: "Ctrl+R"; context: Qt.ApplicationShortcut; enabled: win.dirty; onActivated: win.adopt() }
    Shortcut { sequence: "1"; context: Qt.ApplicationShortcut; onActivated: win.page = 0 }
    Shortcut { sequence: "2"; context: Qt.ApplicationShortcut; onActivated: win.page = 1 }
    Shortcut { sequence: "3"; context: Qt.ApplicationShortcut; onActivated: win.page = 2 }
    Shortcut { sequence: "4"; context: Qt.ApplicationShortcut; onActivated: win.page = 3 }
    Shortcut { sequence: "5"; context: Qt.ApplicationShortcut; onActivated: win.page = 4 }
    Shortcut { sequence: "6"; context: Qt.ApplicationShortcut; onActivated: win.page = 5 }

    // Right-to-left languages mirror the window; the Translator sets the
    // application's layout direction.
    Item {
        anchors.fill: parent
        LayoutMirroring.enabled: Qt.application.layoutDirection === Qt.RightToLeft
        LayoutMirroring.childrenInherit: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            // The daemon is stopped: say so. Everything below is disabled
            // until it answers again, which the backend notices by itself.
            Rectangle {
                visible: !backend.running
                Layout.fillWidth: true
                implicitHeight: stopped.implicitHeight + 24
                color: theme.red
                Txt {
                    id: stopped
                    anchors.fill: parent
                    anchors.margins: 12
                    color: "black"
                    wrapMode: Text.Wrap
                    font.weight: Font.DemiBold
                    text: qsTr("The daemon (avelld) is not running, so nothing can be read or changed. This window comes back by itself when the daemon does. Check it with: systemctl status avelld")
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: 210
                    color: theme.surface

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 4

                        Txt {
                            text: qsTr("Avell Control")
                            font.pixelSize: 18
                            font.weight: Font.DemiBold
                            Layout.margins: 8
                            Layout.bottomMargin: 12
                        }
                        Repeater {
                            model: win.pages
                            Rectangle {
                                id: entry
                                required property string modelData
                                required property int index
                                readonly property bool current: win.page === index
                                Layout.fillWidth: true
                                implicitHeight: 40
                                radius: theme.radius
                                color: current ? theme.accent : hovered.hovered ? theme.border : "transparent"

                                Txt {
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.left: parent.left
                                    anchors.leftMargin: 12
                                    text: entry.modelData
                                    color: entry.current ? theme.accentForeground : theme.foreground
                                }
                                Txt {
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.right: parent.right
                                    anchors.rightMargin: 12
                                    text: entry.index + 1
                                    font.pixelSize: 12
                                    color: entry.current ? theme.accentForeground : theme.muted
                                }
                                HoverHandler { id: hovered }
                                TapHandler { onTapped: win.page = entry.index }
                            }
                        }
                        Item { Layout.fillHeight: true }
                        Txt {
                            text: qsTr("? for the keys")
                            color: theme.muted
                            font.pixelSize: 12
                            Layout.margins: 8
                        }
                    }
                }

                StackLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: win.page
                    enabled: backend.running
                    opacity: backend.running ? 1 : 0.5

                    StatusPage { app: win }
                    // The pages that edit the configuration exist once there
                    // is one to edit.
                    Loader { active: win.hasConfig; sourceComponent: Component { FansPage { app: win } } }
                    Loader { active: win.hasConfig; sourceComponent: Component { ProfilesPage { app: win } } }
                    Loader { active: win.hasConfig; sourceComponent: Component { AutomationPage { app: win } } }
                    BatteryPage { app: win }
                    KeyboardPage { app: win }
                }
            }

            // What the daemon answered to a change that applies at once.
            Rectangle {
                visible: backend.lastError !== ""
                Layout.fillWidth: true
                implicitHeight: failure.implicitHeight + 16
                color: theme.surface
                RowLayout {
                    id: failure
                    anchors.fill: parent
                    anchors.leftMargin: 24
                    anchors.rightMargin: 16
                    spacing: 12
                    Txt {
                        text: backend.lastError
                        color: theme.red
                        Layout.fillWidth: true
                        wrapMode: Text.Wrap
                    }
                    Btn {
                        text: qsTr("Dismiss")
                        onClicked: backend.clearError()
                    }
                }
            }

            // Unsaved edits to the configuration, and what the daemon said to
            // the last attempt to save them.
            Rectangle {
                visible: win.dirty
                Layout.fillWidth: true
                implicitHeight: bar.implicitHeight + 16
                color: theme.surface
                Rectangle { width: parent.width; height: 1; color: theme.border }
                RowLayout {
                    id: bar
                    anchors.fill: parent
                    anchors.leftMargin: 24
                    anchors.rightMargin: 16
                    spacing: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Txt {
                            visible: win.saveError !== ""
                            text: qsTr("Not saved. The daemon answered: %1").arg(win.saveError)
                            color: theme.red
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                        }
                        Txt {
                            text: win.outdated ? qsTr("Unsaved changes. The configuration has changed outside this window since; saving replaces it.")
                                               : qsTr("Unsaved changes")
                            color: win.outdated ? theme.yellow : theme.muted
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                        }
                    }
                    Btn {
                        text: qsTr("Revert")
                        onClicked: win.adopt()
                    }
                    Btn {
                        text: qsTr("Save")
                        primary: true
                        enabled: backend.running && !win.saving
                        onClicked: win.save()
                    }
                }
            }
        }

        Popup {
            id: discard
            anchors.centerIn: parent
            modal: true
            padding: 24
            background: Rectangle { color: theme.surface; border.color: theme.border; radius: theme.radius }
            Overlay.modal: Rectangle { color: "#99000000" }

            ColumnLayout {
                spacing: 20
                Txt {
                    text: qsTr("Discard the unsaved changes and quit?")
                    font.pixelSize: 16
                }
                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 8
                    Btn { text: qsTr("Keep editing"); onClicked: discard.close() }
                    Btn { text: qsTr("Discard and quit"); primary: true; onClicked: Qt.quit() }
                }
            }
        }

        Popup {
            anchors.centerIn: parent
            visible: win.helpVisible
            modal: true
            padding: 24
            background: Rectangle { color: theme.surface; border.color: theme.border; radius: theme.radius }
            Overlay.modal: Rectangle { color: "#99000000" }
            onClosed: win.helpVisible = false

            ColumnLayout {
                spacing: 10
                Repeater {
                    model: [
                        ["1 – 6", qsTr("Go to a page")],
                        ["Ctrl+S", qsTr("Save the configuration")],
                        ["Ctrl+R", qsTr("Revert the unsaved changes")],
                        ["← → ↑ ↓", qsTr("Move the selected point of a fan curve")],
                        ["Delete", qsTr("Remove the selected point")],
                        ["?", qsTr("Show or hide this list")],
                        ["Q", qsTr("Quit")]
                    ]
                    RowLayout {
                        id: key
                        required property var modelData
                        spacing: 16
                        Txt { text: key.modelData[0]; color: theme.accent; Layout.preferredWidth: 90 }
                        Txt { text: key.modelData[1] }
                    }
                }
            }
        }
    }
}
