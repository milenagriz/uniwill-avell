import os, sys, time, subprocess, struct
dur = int(sys.argv[1]); out = sys.argv[2]
H = "/sys/class/hwmon/hwmon8/"
def rd(p):
    try: return open(p).read().strip()
    except OSError: return "?"
def energy(): return int(rd("/sys/class/powercap/intel-rapl:0/energy_uj"))
def freq():
    fs = [int(rd(f"/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_cur_freq")) for i in range(os.cpu_count())]
    return sum(fs) / len(fs) / 1000
def msr(a):
    fd = os.open("/dev/cpu/0/msr", os.O_RDONLY)
    try: return struct.unpack("<Q", os.pread(fd, 8, a))[0]
    finally: os.close(fd)
def ec():
    fd = os.open("/dev/mem", os.O_RDONLY)
    try: return os.pread(fd, 4096, 0xFE410000)
    finally: os.close(fd)
p = subprocess.Popen(["openssl", "speed", "-evp", "sha256", "-multi", str(os.cpu_count()), "-seconds", str(dur), "-bytes", "16384"],
                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
t0 = time.time(); e0 = energy(); tp = t0
print(" t   pkgW  MHz  pkgC ecCPU fan1 fan2 pwm1 pwm2 751 perf_limit_status")
while p.poll() is None:
    time.sleep(3)
    e1 = energy(); t = time.time()
    w = (e1 - e0) / 1e6 / (t - tp); e0 = e1; tp = t
    pl = msr(0x64F) & 0xFFFF
    print(f"{t-t0:4.0f} {w:6.1f} {freq():5.0f} {int(rd('/sys/class/hwmon/hwmon5/temp1_input'))//1000:4d} {int(rd(H+'temp1_input'))//1000:5d} {rd(H+'fan1_input'):>4} {rd(H+'fan2_input'):>4} {rd(H+'pwm1'):>4} {rd(H+'pwm2'):>4}  {ec()[0x751]:02x} {pl:#06x}", flush=True)
open(out, "wb").write(ec())
