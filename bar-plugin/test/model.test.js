// node bar-plugin/test/model.test.js
const assert = require("node:assert")
const M = require("../Model.js")

const good = JSON.stringify({
  profile: "performance", hold: null,
  temperatures: { cpu_c: 63, gpu_c: 44 },
  fans: { mode: "manual", control: "curve", target_percent: 66, fans: [{ rpm: 3963, duty: 168 }, { rpm: 3970, duty: 168 }] },
  gpu: { state: "awake", tgp_offset_w: 20, power_draw_w: 6.88, power_limit_w: 80.0, utilization_percent: 0 },
  cpu: { long_term_w: 80, short_term_w: 80 }, unavailable: []
})

let s = M.parseStatus(good)
assert.strictEqual(s.running, true)
assert.strictEqual(s.profile, "performance")
assert.strictEqual(s.hold, null)
assert.strictEqual(M.hottest(s), 63)
assert.strictEqual(M.fastestRpm(s), 3970)
assert.strictEqual(s.fans[0].dutyPercent, 66)
assert.strictEqual(s.fanControl, "curve")
assert.strictEqual(M.maxFanOn(s), false)
assert.strictEqual(s.gpuDrawW, 6.88)

// A stopped daemon, and anything else that is not a status, shows no values.
for (const raw of ['{"error": "avelld is not running"}', "", "not json", "[]", "null", "{}", '{"profile": 3}', "x".repeat(70000)]) {
  s = M.parseStatus(raw)
  assert.strictEqual(s.running, false, raw.slice(0, 20))
  assert.strictEqual(s.cpuC, null)
  assert.deepStrictEqual(s.fans, [])
}
assert.strictEqual(M.parseStatus('{"error": "avelld is not running"}').error, "avelld is not running")

// Asleep card: no power figures, and a missing GPU temperature leaves the CPU's.
s = M.parseStatus(JSON.stringify({ profile: "balanced", temperatures: { cpu_c: 50, gpu_c: null },
  fans: { mode: "max", control: "max", fans: [] }, gpu: { state: "asleep", tgp_offset_w: 0 } }))
assert.strictEqual(s.gpuState, "asleep")
assert.strictEqual(s.gpuDrawW, null)
assert.strictEqual(M.hottest(s), 50)
assert.strictEqual(M.fastestRpm(s), null)
assert.strictEqual(M.maxFanOn(s), true)

// Untrusted values: wrong types dropped, numbers clamped, words from fixed lists, text bounded.
s = M.parseStatus(JSON.stringify({
  profile: "<img src=x>; rm -rf /", hold: "<b>game</b>\n" + "y".repeat(500),
  temperatures: { cpu_c: "63", gpu_c: 1e9 },
  fans: { mode: {}, control: "$(reboot)", target_percent: 900, fans: [{ rpm: -5, duty: 9999 }, "x", null, { rpm: 1 }, { rpm: 2 }, { rpm: 3 }, { rpm: 4 }] },
  gpu: { state: "exploded", power_draw_w: NaN }, cpu: []
}))
assert.strictEqual(s.running, true)
assert.strictEqual(s.profile, "")
assert.strictEqual(M.isProfile(s.profile), false)
assert.ok(s.hold.startsWith("<b>game</b> y") && s.hold.length <= 120)
assert.strictEqual(s.cpuC, null)
assert.strictEqual(s.gpuC, 150)
assert.strictEqual(s.fanControl, "")
assert.strictEqual(s.targetPercent, 100)
assert.strictEqual(s.fans.length, 4)
assert.deepStrictEqual(s.fans[0], { rpm: 0, duty: 255, dutyPercent: 100 })
assert.strictEqual(s.gpuState, "unknown")
assert.strictEqual(s.cpuLongW, null)

assert.strictEqual(M.parseActionError('{"error": "no such profile"}', 1), "no such profile")
assert.strictEqual(M.parseActionError('{"profile": "balanced"}', 0), "")
assert.strictEqual(M.parseActionError("", 0), "")
assert.notStrictEqual(M.profileIcon("balanced"), M.profileIcon("performance"))

console.log("model: ok")
