import QtQuick
import Quickshell
import qs.Commons
// A bare "Color" can resolve to QtQuick's own value type, not the theme.
import qs.Commons as Commons
import qs.Ui
import "Model.js" as Model

// Bar widget and panel for the Avell A52 MOB control stack.
//
// A thin client: every value comes from "avellctl --json status" and every
// action is an avellctl command (see Poller.qml). The plugin holds no machine
// logic and never touches the graphics card itself.
Panel {
  id: root

  moduleName: "avell.control"
  ipcTarget: "avell.control"
  // The panel owns the IPC handler itself, to add the two actions to it:
  //   omarchy-shell avell.control setProfile balanced
  //   omarchy-shell avell.control toggleMaxFan
  manageIpc: false

  ShellIpc {
    target: root.ipcTarget

    function open(): void { root.open() }
    function close(): void { root.close() }
    function show(): void { root.open() }
    function hide(): void { root.close() }
    function toggle(): void { root.toggle() }
    function setProfile(name: string): void { root.setProfile(name) }
    function toggleMaxFan(): void { root.toggleMaxFan() }
  }

  readonly property color fg: bar ? bar.foreground : Commons.Color.foreground
  readonly property color dim: Qt.darker(fg, 1.4)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family
  readonly property bool vertical: bar ? bar.vertical : false
  readonly property int barSize: bar ? bar.barSize : Style.bar.sizeHorizontal
  readonly property bool showLabel: !vertical && setting("showLabel", true) !== false

  // ---------------- data: one shared poller ------------------------------
  //
  // The plugin's service polls once for every monitor's widget. Under a bar
  // that does not hand the service over, the widget polls for itself.
  property var service: null
  readonly property var source: service ? service : localPoller
  readonly property var st: source.status
  readonly property bool loaded: source.loaded === true
  readonly property bool running: loaded && st.running === true
  readonly property bool maxFan: running && Model.maxFanOn(st)

  function lookupService() {
    if (!bar || !bar.shell || typeof bar.shell.serviceFor !== "function") return null
    var found = bar.shell.serviceFor(moduleName)
    return found && typeof found.attach === "function" ? found : null
  }

  function resolveService() {
    var found = lookupService()
    if (found === service) return
    releaseService()
    service = found
    if (found) {
      pollLocally = false
      found.attach()
    }
  }

  function releaseService() {
    var old = service
    service = null
    try {
      if (old) old.detach()
    } catch (e) {
    }
  }

  onBarChanged: resolveService()
  Component.onCompleted: resolveService()
  Component.onDestruction: releaseService()

  // The service can be mounted after the widget, so the widget looks again
  // before it starts polling for itself, and keeps looking after that.
  property bool pollLocally: false

  Poller {
    id: localPoller
    active: root.service === null && root.pollLocally
  }

  Timer {
    interval: 2000
    running: root.service === null
    repeat: true
    onTriggered: {
      root.resolveService()
      root.pollLocally = root.service === null
    }
  }

  // ---------------- formatting -------------------------------------------

  Commons.Translations { id: strings; directory: Qt.resolvedUrl("i18n") }

  readonly property string dash: "—"

  function n(value, decimals) {
    return Commons.I18n.formatNumber(value, decimals)
  }

  function celsius(value) {
    return typeof value === "number" ? strings.t("unit.celsius").arg(n(Math.round(value))) : dash
  }

  function watts(value, decimals) {
    return typeof value === "number" ? strings.t("unit.watts").arg(n(value, decimals)) : dash
  }

  function profileLabel(name) {
    return Model.isProfile(name) ? strings.t("profile." + name) : strings.t("profile.unknown")
  }

  function controlLabel(status) {
    var control = status.fanControl
    if (control === "") return dash
    var target = status.targetPercent
    if ((control === "curve" || control === "fixed") && typeof target === "number")
      return strings.t("fans.control." + control + "Target").arg(n(target))
    return strings.t("fans.control." + control)
  }

  function fanReading(fan) {
    if (!fan || typeof fan.rpm !== "number") return dash
    if (typeof fan.dutyPercent !== "number") return strings.t("fans.rpmOnly").arg(n(fan.rpm))
    return strings.t("fans.reading").arg(n(fan.rpm)).arg(n(fan.dutyPercent))
  }

  // The fan speed the bar shows: the faster fan's rpm, else the target percent.
  readonly property string fanSpeedText: {
    if (!running) return ""
    var rpm = Model.fastestRpm(st)
    if (rpm !== null) return strings.t("unit.rpm").arg(n(rpm))
    if (typeof st.targetPercent === "number") return strings.t("unit.percent").arg(n(st.targetPercent))
    return ""
  }

  readonly property string hottestText: running && Model.hottest(st) !== null ? celsius(Model.hottest(st)) : ""

  readonly property string gpuPowerText: {
    if (typeof st.gpuDrawW !== "number") return dash
    if (typeof st.gpuLimitW !== "number") return watts(st.gpuDrawW, 1)
    return strings.t("gpu.powerOfLimit").arg(n(st.gpuDrawW, 1)).arg(n(st.gpuLimitW, 0))
  }

  readonly property string cpuLimitsText: {
    if (typeof st.cpuLongW !== "number" && typeof st.cpuShortW !== "number") return dash
    if (typeof st.cpuLongW !== "number" || typeof st.cpuShortW !== "number")
      return watts(typeof st.cpuLongW === "number" ? st.cpuLongW : st.cpuShortW, 0)
    return strings.t("cpu.limitsValue").arg(n(st.cpuLongW, 0)).arg(n(st.cpuShortW, 0))
  }

  readonly property string barIcon: running ? Model.profileIcon(st.profile) : Model.stoppedIcon()

  readonly property string barLabel: {
    var parts = []
    if (hottestText !== "") parts.push(hottestText)
    if (fanSpeedText !== "") parts.push(fanSpeedText)
    return parts.join("  ")
  }

  readonly property string barTooltip: {
    if (!loaded) return strings.t("loading")
    if (!running) return strings.t("stopped.title")
    var lines = [
      strings.t("tooltip.profile").arg(profileLabel(st.profile)),
      strings.t("tooltip.temps").arg(celsius(st.cpuC)).arg(celsius(st.gpuC))
    ]
    if (fanSpeedText !== "") lines.push(strings.t("tooltip.fans").arg(fanSpeedText).arg(controlLabel(st)))
    if (st.hold) lines.push(strings.t("tooltip.hold").arg(st.hold))
    return lines.join("\n")
  }

  // ---------------- actions and keyboard cursor --------------------------

  // 0..2 are the Power profile buttons, 3 is the Max fan toggle.
  property int cursorIndex: 0
  property bool cursorActive: false

  function setProfile(name) {
    if (running) source.setProfile(name)
  }

  function toggleMaxFan() {
    if (running) source.setMaxFan(!maxFan)
  }

  function moveCursor(dx, dy) {
    if (dy > 0) cursorIndex = 3
    else if (dy < 0) cursorIndex = cursorIndex === 3 ? Math.max(0, Model.PROFILES.indexOf(st.profile)) : cursorIndex
    else if (cursorIndex < 3) cursorIndex = Model.clampIndex(cursorIndex + dx, 3)
  }

  function activateCursor() {
    if (cursorIndex === 3) toggleMaxFan()
    else setProfile(Model.PROFILES[cursorIndex])
  }

  onOpenedChanged: {
    if (!opened) return
    source.refresh()
    cursorIndex = Math.max(0, Model.PROFILES.indexOf(st.profile))
    cursorActive = false
  }

  implicitWidth: button.item ? button.item.implicitWidth : 0
  implicitHeight: button.item ? button.item.implicitHeight : barSize

  // ---------------- bar button -------------------------------------------

  Loader {
    id: button
    anchors.fill: parent
    sourceComponent: root.showLabel && root.barLabel !== "" ? labelledButton : iconButton
  }

  Component {
    id: iconButton

    BarIconButton {
      anchors.fill: parent
      bar: root.bar
      text: root.barIcon
      dimmed: !root.running
      tooltipText: root.barTooltip
      onPressed: root.toggle()
    }
  }

  Component {
    id: labelledButton

    WidgetButton {
      anchors.fill: parent
      bar: root.bar
      text: root.barIcon + "  " + root.barLabel
      tooltipText: root.barTooltip
      onPressed: root.toggle()
    }
  }

  // ---------------- panel ------------------------------------------------

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: panel.loadedContent
    contentWidth: panel.fittedContentWidth(Style.space(380))
    contentHeight: panel.fittedContentHeight(panel.loadedContentHeight)

    content: PanelKeyCatcher {
      anchors.fill: parent
      implicitHeight: column.implicitHeight
      onMoveRequested: function(dx, dy) {
        if (!root.cursorActive) { root.cursorActive = true; return }
        root.moveCursor(dx, dy)
      }
      onActivateRequested: if (root.cursorActive) root.activateCursor()
      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }

      Column {
        id: column
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Style.space(14)

        // ---------- Hero: profile icon, title, hottest temperature ----------
        Item {
          width: parent.width
          implicitHeight: Math.max(heroIcon.implicitHeight, heroLabels.implicitHeight, heroTemp.implicitHeight)

          Text {
            id: heroIcon
            textFormat: Text.PlainText
            text: root.barIcon
            color: root.fg
            opacity: root.running ? 1 : 0.45
            font.family: root.fontFamily
            font.pixelSize: Style.font.display
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
          }

          Column {
            id: heroLabels
            anchors.left: heroIcon.right
            anchors.leftMargin: Style.space(14)
            anchors.right: heroTemp.left
            anchors.rightMargin: Style.space(10)
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.space(2)

            Text {
              textFormat: Text.PlainText
              text: !root.loaded ? strings.t("loading")
                : (root.running ? root.profileLabel(root.st.profile) : strings.t("stopped.title"))
              color: root.fg
              font.family: root.fontFamily
              font.pixelSize: Style.font.title
              font.bold: true
              elide: Text.ElideRight
              width: parent.width
            }

            Text {
              textFormat: Text.PlainText
              visible: root.loaded
              text: root.running ? strings.t("profile.header").toUpperCase() : strings.t("stopped.hint")
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
              font.bold: root.running
              font.letterSpacing: root.running ? 1.2 : 0
              wrapMode: Text.WordWrap
              width: parent.width
            }
          }

          Text {
            id: heroTemp
            textFormat: Text.PlainText
            text: root.hottestText
            color: root.fg
            font.family: root.fontFamily
            font.pixelSize: Style.font.displayLarge
            font.bold: true
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
          }
        }

        // Everything below shows live values only: with avelld stopped it
        // goes away rather than keeping the last reading on screen.

        // ---------- Power profile ----------
        PanelSeparator { visible: root.running; foreground: root.fg }

        Column {
          visible: root.running
          width: parent.width
          spacing: Style.space(10)

          PanelSectionHeader {
            text: strings.t("profile.header")
            foreground: root.fg
            fontFamily: root.fontFamily
          }

          Row {
            id: profileRow
            width: parent.width
            spacing: Style.space(6)
            readonly property real cellWidth: (width - spacing * 2) / 3

            Repeater {
              model: Model.PROFILES

              Button {
                required property var modelData
                required property int index
                width: profileRow.cellWidth
                iconText: Model.profileIcon(String(modelData))
                iconSize: Style.font.title
                text: root.profileLabel(String(modelData))
                fontSize: Style.font.bodySmall
                foreground: root.fg
                fontFamily: root.fontFamily
                verticalPadding: Style.spacing.controlPaddingY + Style.space(2)
                bordered: true
                active: root.st.profile === modelData
                selected: root.st.profile === modelData
                hasCursor: root.cursorActive && root.cursorIndex === index
                onClicked: root.setProfile(String(modelData))
                onHovered: function(h) {
                  if (h) {
                    root.cursorActive = true
                    root.cursorIndex = index
                  }
                }
              }
            }
          }

          InfoPair {
            label: strings.t("hold.label")
            value: root.st.hold ? root.st.hold : strings.t("hold.none")
          }
        }

        // ---------- Fans ----------
        PanelSeparator { visible: root.running; foreground: root.fg }

        Column {
          visible: root.running
          width: parent.width
          spacing: Style.space(10)

          PanelSectionHeader {
            text: strings.t("fans.header")
            foreground: root.fg
            fontFamily: root.fontFamily
          }

          Toggle {
            width: parent.width
            label: strings.t("fans.maxFan")
            description: strings.t("fans.maxFanDescription")
            checked: root.maxFan
            hasCursor: root.cursorActive && root.cursorIndex === 3
            foreground: root.fg
            fontFamily: root.fontFamily
            activeFocusOnTab: false
            onClicked: root.toggleMaxFan()
            onHovered: function(h) {
              if (h) {
                root.cursorActive = true
                root.cursorIndex = 3
              }
            }
          }

          Column {
            width: parent.width
            spacing: Style.spacing.labelGap

            InfoPair { label: strings.t("fans.control"); value: root.controlLabel(root.st) }

            Repeater {
              model: root.st.fans

              InfoPair {
                required property var modelData
                required property int index
                label: strings.t("fans.fan").arg(root.n(index + 1))
                value: root.fanReading(modelData)
              }
            }
          }
        }

        // ---------- Temperatures and power ----------
        PanelSeparator { visible: root.running; foreground: root.fg }

        Column {
          visible: root.running
          width: parent.width
          spacing: Style.spacing.labelGap

          PanelSectionHeader {
            text: strings.t("temps.header")
            foreground: root.fg
            fontFamily: root.fontFamily
          }
          InfoPair { label: strings.t("temps.cpu"); value: root.celsius(root.st.cpuC) }
          InfoPair { label: strings.t("temps.gpu"); value: root.celsius(root.st.gpuC) }
        }

        Column {
          visible: root.running
          width: parent.width
          spacing: Style.spacing.labelGap

          PanelSectionHeader {
            text: strings.t("gpu.header")
            foreground: root.fg
            fontFamily: root.fontFamily
          }
          InfoPair { label: strings.t("gpu.state"); value: strings.t("gpu.state." + root.st.gpuState) }
          InfoPair { label: strings.t("gpu.power"); value: root.gpuPowerText }
          InfoPair { label: strings.t("gpu.tgpOffset"); value: root.watts(root.st.tgpOffsetW, 0) }
        }

        Column {
          visible: root.running
          width: parent.width
          spacing: Style.spacing.labelGap

          PanelSectionHeader {
            text: strings.t("cpu.header")
            foreground: root.fg
            fontFamily: root.fontFamily
          }
          InfoPair { label: strings.t("cpu.limits"); value: root.cpuLimitsText }
        }

        // What avellctl answered to the last action, when it refused.
        Text {
          textFormat: Text.PlainText
          visible: root.running && text !== ""
          text: String(root.source.actionError || "")
          color: root.bar ? root.bar.urgent : Commons.Color.urgent
          font.family: root.fontFamily
          font.pixelSize: Style.font.bodySmall
          wrapMode: Text.WordWrap
          width: parent.width
        }
      }
    }
  }

  component InfoPair: Item {
    property string label: ""
    property string value: ""

    width: parent ? parent.width : 0
    implicitHeight: Math.max(pairLabel.implicitHeight, pairValue.implicitHeight)

    Text {
      id: pairLabel
      textFormat: Text.PlainText
      text: label
      color: root.fg
      opacity: 0.6
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
    }

    Text {
      id: pairValue
      textFormat: Text.PlainText
      text: value
      color: root.fg
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      horizontalAlignment: Text.AlignRight
      elide: Text.ElideRight
      anchors.left: pairLabel.right
      anchors.leftMargin: Style.space(8)
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
    }
  }
}
