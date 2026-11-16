import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// A slider for something the daemon sets at once (no Save): it shows the
// machine's real value, and sends the owner's value once the handle rests.
Slider {
    id: slider
    property real machine: 0
    signal committed(int value)

    stepSize: 1
    snapMode: Slider.SnapAlways
    Layout.preferredWidth: 260

    // The machine's value, except while the owner holds the handle and until
    // the daemon has had the time to apply what was sent.
    Binding {
        target: slider
        property: "value"
        value: slider.machine
        when: !slider.pressed && !commit.running && !settle.running
        restoreMode: Binding.RestoreNone
    }
    onMoved: commit.restart()
    Timer {
        id: commit
        interval: 250
        onTriggered: {
            if (slider.pressed) {
                restart()
            } else {
                settle.restart()
                slider.committed(Math.round(slider.value))
            }
        }
    }
    Timer { id: settle; interval: 1500 }
}
