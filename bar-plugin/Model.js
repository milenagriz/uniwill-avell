// Pure helpers for the Avell Control bar plugin: no QML, no I/O.
// Everything avellctl prints is treated as untrusted: parsed in try/catch,
// type-checked, clamped and length-limited before the UI sees it.

var PROFILES = ["power-saver", "balanced", "performance"]
var FAN_CONTROLS = ["curve", "fixed", "max", "automatic"]
var GPU_STATES = ["awake", "asleep", "unknown"]
var MAX_RAW = 65536

function isObject(v) {
  return v !== null && typeof v === "object" && !Array.isArray(v)
}

// A finite number inside [lo, hi], or null.
function num(v, lo, hi) {
  if (typeof v !== "number" || !isFinite(v)) return null
  return Math.max(lo, Math.min(hi, v))
}

// One line of plain text: control characters dropped, length capped.
function cleanText(v, max) {
  if (typeof v !== "string") return ""
  var s = v.replace(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, " ").trim()
  var limit = max > 0 ? max : 120
  return s.length > limit ? s.substring(0, limit - 1) + "…" : s
}

function oneOf(v, allowed) {
  return typeof v === "string" && allowed.indexOf(v) !== -1 ? v : ""
}

function unavailable(error) {
  return {
    running: false,
    error: cleanText(error, 200),
    profile: "",
    hold: null,
    cpuC: null,
    gpuC: null,
    fanMode: "",
    fanControl: "",
    targetPercent: null,
    fans: [],
    gpuState: "unknown",
    tgpOffsetW: null,
    gpuDrawW: null,
    gpuLimitW: null,
    gpuUtilPercent: null,
    cpuLongW: null,
    cpuShortW: null
  }
}

// "avellctl --json status" output to the plugin's view of the machine.
// Anything that is not the expected status object (empty output, broken JSON,
// {"error": ...} from a stopped daemon) gives running: false and no values.
function parseStatus(raw) {
  var text = typeof raw === "string" ? raw : ""
  if (text.length === 0 || text.length > MAX_RAW) return unavailable("")
  var data
  try {
    data = JSON.parse(text)
  } catch (e) {
    return unavailable("")
  }
  if (!isObject(data)) return unavailable("")
  if (data.error !== undefined) return unavailable(typeof data.error === "string" ? data.error : "")
  if (typeof data.profile !== "string") return unavailable("")

  var temps = isObject(data.temperatures) ? data.temperatures : {}
  var fans = isObject(data.fans) ? data.fans : {}
  var gpu = isObject(data.gpu) ? data.gpu : {}
  var cpu = isObject(data.cpu) ? data.cpu : {}

  var fanList = []
  var rawFans = Array.isArray(fans.fans) ? fans.fans : []
  for (var i = 0; i < rawFans.length && fanList.length < 4; i++) {
    var f = rawFans[i]
    if (!isObject(f)) continue
    var rpm = num(f.rpm, 0, 20000)
    var duty = num(f.duty, 0, 255)
    if (rpm === null && duty === null) continue
    fanList.push({
      rpm: rpm === null ? null : Math.round(rpm),
      duty: duty === null ? null : Math.round(duty),
      dutyPercent: duty === null ? null : Math.round(duty * 100 / 255)
    })
  }

  var hold = cleanText(data.hold, 120)
  var target = num(fans.target_percent, 0, 100)
  var out = unavailable("")
  out.running = true
  out.profile = oneOf(data.profile, PROFILES)
  out.hold = hold === "" ? null : hold
  out.cpuC = num(temps.cpu_c, -40, 150)
  out.gpuC = num(temps.gpu_c, -40, 150)
  out.fanMode = oneOf(fans.mode, ["max", "manual", "automatic"])
  out.fanControl = oneOf(fans.control, FAN_CONTROLS)
  out.targetPercent = target === null ? null : Math.round(target)
  out.fans = fanList
  out.gpuState = oneOf(gpu.state, GPU_STATES) || "unknown"
  out.tgpOffsetW = num(gpu.tgp_offset_w, 0, 500)
  out.gpuDrawW = num(gpu.power_draw_w, 0, 1000)
  out.gpuLimitW = num(gpu.power_limit_w, 0, 1000)
  out.gpuUtilPercent = num(gpu.utilization_percent, 0, 100)
  out.cpuLongW = num(cpu.long_term_w, 0, 1000)
  out.cpuShortW = num(cpu.short_term_w, 0, 1000)
  return out
}

// The error an action ("avellctl --json profile set ...") reported, or "".
function parseActionError(raw, exitCode) {
  var text = typeof raw === "string" ? raw : ""
  if (text.length > 0 && text.length <= MAX_RAW) {
    try {
      var data = JSON.parse(text)
      if (isObject(data) && typeof data.error === "string") return cleanText(data.error, 200)
    } catch (e) {
    }
  }
  return exitCode === 0 ? "" : cleanText(text, 200)
}

// The hotter of the two temperatures, or null when neither is known.
function hottest(status) {
  var s = status || {}
  if (typeof s.cpuC !== "number") return typeof s.gpuC === "number" ? s.gpuC : null
  if (typeof s.gpuC !== "number") return s.cpuC
  return Math.max(s.cpuC, s.gpuC)
}

// The rpm of the faster fan, or null when no fan reports one.
function fastestRpm(status) {
  var fans = status && Array.isArray(status.fans) ? status.fans : []
  var best = null
  for (var i = 0; i < fans.length; i++) {
    var rpm = fans[i] ? fans[i].rpm : null
    if (typeof rpm === "number" && (best === null || rpm > best)) best = rpm
  }
  return best
}

function maxFanOn(status) {
  var s = status || {}
  return s.fanControl === "max" || s.fanMode === "max"
}

function isProfile(name) {
  return PROFILES.indexOf(name) !== -1
}

// Nerd Font glyphs (Material Design Icons range).
function profileIcon(name) {
  if (name === "power-saver") return String.fromCodePoint(0xF032A) // leaf
  if (name === "balanced") return String.fromCodePoint(0xF05D1)    // scale-balance
  if (name === "performance") return String.fromCodePoint(0xF04C5) // speedometer
  return String.fromCodePoint(0xF0210)                             // fan
}

function stoppedIcon() {
  return String.fromCodePoint(0xF081D) // fan-off
}

function clampIndex(index, length) {
  if (length <= 0) return 0
  return Math.max(0, Math.min(length - 1, index))
}

if (typeof module !== "undefined") {
  module.exports = {
    PROFILES: PROFILES,
    num: num,
    cleanText: cleanText,
    unavailable: unavailable,
    parseStatus: parseStatus,
    parseActionError: parseActionError,
    hottest: hottest,
    fastestRpm: fastestRpm,
    maxFanOn: maxFanOn,
    isProfile: isProfile,
    profileIcon: profileIcon,
    stoppedIcon: stoppedIcon,
    clampIndex: clampIndex
  }
}
