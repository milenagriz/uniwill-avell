import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

// A fan curve as a chart: temperature across, fan speed up. Points are
// dragged, added (double-click, or the button) and removed (right-click,
// Delete, or the button). It does not judge the curve: the points keep their
// order and go where they are put, and the daemon says whether it is valid.
Item {
    id: editor
    property var points: []         // [[°C, percent], ...]
    property int selected: -1
    property var markerTemp         // the temperature now, when this curve is in use
    property var markerPercent
    signal edited(var points)

    readonly property int tMin: 20
    readonly property int tMax: 110
    readonly property real padLeft: 46
    readonly property real padRight: 16
    readonly property real padTop: 12
    readonly property real padBottom: 26
    readonly property real plotWidth: Math.max(1, chart.width - padLeft - padRight)
    readonly property real plotHeight: Math.max(1, chart.height - padTop - padBottom)
    readonly property bool hasSelection: selected >= 0 && selected < points.length
    readonly property var selectedPoint: hasSelection ? points[selected] : [0, 0]

    implicitHeight: 380
    activeFocusOnTab: true

    function xFor(t) { return padLeft + (Math.max(tMin, Math.min(tMax, t)) - tMin) / (tMax - tMin) * plotWidth }
    function yFor(p) { return padTop + (1 - Math.max(0, Math.min(100, p)) / 100) * plotHeight }
    function tAt(x) { return Math.max(tMin, Math.min(tMax, Math.round(tMin + (x - padLeft) / plotWidth * (tMax - tMin)))) }
    function pAt(y) { return Math.max(0, Math.min(100, Math.round((1 - (y - padTop) / plotHeight) * 100))) }
    function copy() { return points.map(p => [p[0], p[1]]) }

    function move(index, t, p) {
        if (index < 0 || index >= points.length)
            return
        var next = copy()
        next[index] = [t, p]
        edited(next)
    }
    function add(t, p) {
        var next = copy()
        var at = next.length
        for (var i = 0; i < next.length; i++) {
            if (next[i][0] > t) { at = i; break }
        }
        next.splice(at, 0, [t, p])
        edited(next)
        selected = at
    }
    // Under the pointer when it is over the chart; else halfway to the next
    // point from the selected one (or the last).
    function addSomewhere() {
        if (hover.hovered) {
            add(tAt(hover.point.position.x), pAt(hover.point.position.y))
            return
        }
        if (points.length === 0) {
            add(60, 50)
            return
        }
        var i = hasSelection ? selected : points.length - 1
        var here = points[i]
        var after = i + 1 < points.length ? points[i + 1] : [Math.min(tMax, here[0] + 10), here[1]]
        add(Math.round((here[0] + after[0]) / 2), Math.round((here[1] + after[1]) / 2))
    }
    function remove(index) {
        if (index < 0 || index >= points.length)
            return
        var next = copy()
        next.splice(index, 1)
        selected = Math.min(index, next.length - 1)
        edited(next)
    }

    onPointsChanged: canvas.requestPaint()
    Connections {
        target: theme
        function onChanged() { canvas.requestPaint() }
    }

    Keys.onPressed: (event) => {
        if (!hasSelection)
            return
        var p = selectedPoint
        if (event.key === Qt.Key_Left) move(selected, Math.max(tMin, p[0] - 1), p[1])
        else if (event.key === Qt.Key_Right) move(selected, Math.min(tMax, p[0] + 1), p[1])
        else if (event.key === Qt.Key_Up) move(selected, p[0], Math.min(100, p[1] + 1))
        else if (event.key === Qt.Key_Down) move(selected, p[0], Math.max(0, p[1] - 1))
        else if (event.key === Qt.Key_Delete || event.key === Qt.Key_Backspace) remove(selected)
        else return
        event.accepted = true
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Item {
            id: chart
            Layout.fillWidth: true
            Layout.fillHeight: true

            Canvas {
                id: canvas
                anchors.fill: parent
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onPaint: {
                    var ctx = getContext("2d")
                    ctx.reset()
                    ctx.font = "11px sans-serif"
                    ctx.lineWidth = 1
                    ctx.strokeStyle = theme.border
                    ctx.fillStyle = theme.muted
                    ctx.textAlign = "right"
                    ctx.textBaseline = "middle"
                    for (var p = 0; p <= 100; p += 25) {
                        var y = Math.round(editor.yFor(p)) + 0.5
                        ctx.beginPath()
                        ctx.moveTo(editor.padLeft, y)
                        ctx.lineTo(editor.padLeft + editor.plotWidth, y)
                        ctx.stroke()
                        ctx.fillText(p + " %", editor.padLeft - 8, y)
                    }
                    ctx.textAlign = "center"
                    ctx.textBaseline = "top"
                    for (var t = editor.tMin; t <= editor.tMax; t += 10) {
                        var x = Math.round(editor.xFor(t)) + 0.5
                        ctx.beginPath()
                        ctx.moveTo(x, editor.padTop)
                        ctx.lineTo(x, editor.padTop + editor.plotHeight)
                        ctx.stroke()
                        ctx.fillText(t + " °C", x, editor.padTop + editor.plotHeight + 7)
                    }

                    // The curve as the daemon reads it: flat before the first
                    // point and after the last, straight lines between.
                    var pts = editor.points
                    if (pts.length > 0) {
                        ctx.beginPath()
                        ctx.moveTo(editor.xFor(editor.tMin), editor.yFor(pts[0][1]))
                        for (var i = 0; i < pts.length; i++)
                            ctx.lineTo(editor.xFor(pts[i][0]), editor.yFor(pts[i][1]))
                        ctx.lineTo(editor.xFor(editor.tMax), editor.yFor(pts[pts.length - 1][1]))
                        ctx.strokeStyle = theme.accent
                        ctx.lineWidth = 2
                        ctx.stroke()
                        ctx.lineTo(editor.xFor(editor.tMax), editor.yFor(0))
                        ctx.lineTo(editor.xFor(editor.tMin), editor.yFor(0))
                        ctx.closePath()
                        ctx.globalAlpha = 0.12
                        ctx.fillStyle = theme.accent
                        ctx.fill()
                        ctx.globalAlpha = 1
                    }
                }
            }

            // Where the machine is now: the temperature, and the speed the
            // daemon is asking for.
            Rectangle {
                visible: typeof editor.markerTemp === "number"
                x: visible ? Math.round(editor.xFor(editor.markerTemp)) : 0
                y: editor.padTop
                width: 1
                height: editor.plotHeight
                color: theme.yellow
            }
            Rectangle {
                visible: typeof editor.markerTemp === "number" && typeof editor.markerPercent === "number"
                x: visible ? Math.round(editor.xFor(editor.markerTemp)) - 4 : 0
                y: visible ? editor.yFor(editor.markerPercent) - 4 : 0
                width: 9
                height: 9
                radius: 4.5
                color: theme.yellow
            }

            HoverHandler { id: hover }

            MouseArea {
                anchors.fill: parent
                onPressed: {
                    editor.forceActiveFocus()
                    editor.selected = -1
                }
                onDoubleClicked: (mouse) => editor.add(editor.tAt(mouse.x), editor.pAt(mouse.y))
            }

            Repeater {
                // The count, not the points: the handles survive a drag, which
                // replaces the points on every move.
                model: editor.points.length
                Item {
                    id: handle
                    required property int index
                    readonly property var point: editor.points[index] || [0, 0]
                    readonly property bool current: editor.selected === index
                    x: editor.xFor(point[0]) - width / 2
                    y: editor.yFor(point[1]) - height / 2
                    width: 30
                    height: 30

                    Rectangle {
                        anchors.centerIn: parent
                        width: handle.current ? 16 : 12
                        height: width
                        radius: width / 2
                        color: handle.current ? theme.accent : theme.surface
                        border.color: theme.accent
                        border.width: 2
                    }
                    MouseArea {
                        anchors.fill: parent
                        preventStealing: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        cursorShape: Qt.SizeAllCursor
                        onPressed: (mouse) => {
                            editor.forceActiveFocus()
                            editor.selected = handle.index
                            // Later: removing destroys this handle.
                            if (mouse.button === Qt.RightButton)
                                Qt.callLater(editor.remove, handle.index)
                        }
                        onPositionChanged: (mouse) => {
                            if (!(mouse.buttons & Qt.LeftButton))
                                return
                            var at = mapToItem(chart, mouse.x, mouse.y)
                            editor.move(handle.index, editor.tAt(at.x), editor.pAt(at.y))
                        }
                    }
                }
            }
        }

        RowLayout {
            // A fixed height, so the chart does not resize under the pointer
            // when a point is selected and the fields appear.
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            Layout.maximumHeight: 56
            spacing: 8

            Txt {
                visible: !editor.hasSelection
                color: theme.muted
                text: qsTr("Drag a point to move it. Double-click the chart to add one, right-click a point to remove it.")
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
            Txt {
                visible: editor.hasSelection
                text: qsTr("Point %1").arg(editor.selected + 1)
            }
            SpinBox {
                visible: editor.hasSelection
                from: editor.tMin
                to: editor.tMax
                editable: true
                value: editor.selectedPoint[0]
                onValueModified: editor.move(editor.selected, value, editor.selectedPoint[1])
            }
            Txt { visible: editor.hasSelection; text: qsTr("°C"); color: theme.muted }
            SpinBox {
                visible: editor.hasSelection
                from: 0
                to: 100
                editable: true
                value: editor.selectedPoint[1]
                onValueModified: editor.move(editor.selected, editor.selectedPoint[0], value)
            }
            Txt { visible: editor.hasSelection; text: qsTr("%"); color: theme.muted }
            Item { visible: editor.hasSelection; Layout.fillWidth: true }
            Btn {
                text: qsTr("Add point")
                onClicked: editor.addSomewhere()
            }
            Btn {
                text: qsTr("Remove point")
                enabled: editor.hasSelection
                onClicked: editor.remove(editor.selected)
            }
        }
    }
}
