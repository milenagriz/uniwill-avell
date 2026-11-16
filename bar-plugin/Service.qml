import QtQuick

// The one poller every bar shares: the widget on each monitor reads this
// service instead of running avellctl itself. It polls only while at least
// one widget is attached.
Poller {
  id: service

  // Injected by omarchy-shell's service loader.
  property string omarchyPath: ""
  property var shell: null
  property var manifest: null

  property int clients: 0

  function attach() { clients += 1 }
  function detach() { clients = Math.max(0, clients - 1) }

  active: clients > 0
}
