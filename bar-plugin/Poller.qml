import QtQuick
import Quickshell.Io
import "Model.js" as Model

// The plugin's only contact with the machine: it runs avellctl, nothing else.
// "avellctl --json status" every 3 s while active, and the two actions the
// panel offers. It never touches the graphics card: avellctl status asks the
// card only while it is awake, so polling does not wake it.
//
// Every command is an argument array with words from fixed lists; nothing
// read from avellctl is ever passed back to a command line.
Item {
  id: poller

  property bool active: false
  // Model.parseStatus()'s shape. running: false until the first answer, and
  // again whenever avelld does not answer, so no stale value is ever shown.
  property var status: Model.unavailable("")
  property bool loaded: false
  property string actionError: ""
  readonly property bool busy: actionProc.running

  property bool sawOutput: false
  property double startedMs: 0

  visible: false

  function refresh() {
    if (statusProc.running) {
      // A status call that hangs must not freeze the last values on screen.
      if (Date.now() - startedMs < 8000) return
      statusProc.running = false
      apply("")
      return
    }
    sawOutput = false
    startedMs = Date.now()
    statusProc.running = true
  }

  function apply(raw) {
    status = Model.parseStatus(raw)
    loaded = true
  }

  function runAction(args) {
    if (actionProc.running) return false
    actionError = ""
    actionProc.command = ["avellctl", "--json"].concat(args)
    actionProc.running = true
    return true
  }

  function setProfile(name) {
    var index = Model.PROFILES.indexOf(String(name))
    if (index < 0) return false
    return runAction(["profile", "set", Model.PROFILES[index]])
  }

  function setMaxFan(on) {
    return runAction(["fan", "set", on === true ? "max" : "curve"])
  }

  onActiveChanged: if (active) refresh()

  Process {
    id: statusProc
    command: ["avellctl", "--json", "status"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        poller.sawOutput = true
        poller.apply(text)
      }
    }
    onExited: function(exitCode) {
      if (exitCode !== 0 && !poller.sawOutput) poller.apply("")
    }
  }

  Process {
    id: actionProc
    stdout: StdioCollector {
      id: actionOut
      waitForEnd: true
    }
    onExited: function(exitCode) {
      poller.actionError = Model.parseActionError(actionOut.text, exitCode)
      poller.refresh()
    }
  }

  Timer {
    interval: 3000
    running: poller.active
    repeat: true
    triggeredOnStart: true
    onTriggered: poller.refresh()
  }
}
