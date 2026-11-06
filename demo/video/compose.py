# Assembles the setup video: title card, the SmartScreen chapter (recorded on
# Windows 11), the setup chapters (recorded on a clean cloud machine), and the
# end card. Captions and chapter tags are the designed overlays from
# design.py, cut together with short crossfades.
# Usage: python -I compose.py <smartscreen dir> <cloud dir> <out.mp4>
# The SmartScreen footage is in footage/ (recorded with record-smartscreen.ps1);
# the cloud footage is the setup-recording artifact of a demo-video run. An
# edits.json beside the cloud captions can trim static waits and move captions.
import json, os, subprocess, sys
from pathlib import Path
from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
import design

FF = os.environ.get("FFMPEG", "ffmpeg")
FFPROBE = os.environ.get("FFPROBE", "ffprobe")
ss_dir, cloud_dir, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
work = out.parent / "work"
work.mkdir(parents=True, exist_ok=True)
FADE = 0.45


def run(*args):
    subprocess.run([FF, "-y", "-loglevel", "error", *map(str, args)], check=True)


def duration(path):
    r = subprocess.run([FFPROBE, "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)],
                       capture_output=True, text=True, check=True)
    return float(r.stdout.strip())


def load(path):
    caps = json.loads(path.read_text(encoding="utf-8-sig"))
    return caps if isinstance(caps, list) else [caps]


# How each chapter is named on screen; the title card lists them in order.
DISPLAY = {"Badge printer": "Choose your printer"}

# Chapter numbers follow the order chapters first appear across both parts.
order = []


def number(chapter):
    if chapter not in order:
        order.append(chapter)
    return order.index(chapter) + 1


def ease(u):
    u = min(max(u, 0.0), 1.0)
    return 4 * u ** 3 if u < 0.5 else 1 - (-2 * u + 2) ** 3 / 2


class Camera:
    """Zoom keyframes (source time, zoom, centre x, centre y), eased between.
    Zoom moves in log space so a push-in feels even all the way."""

    def __init__(self, keys):
        self.keys = sorted(keys)

    def at(self, t):
        ks = self.keys
        if t <= ks[0][0]:
            return ks[0][1:]
        for (t0, z0, x0, y0), (t1, z1, x1, y1) in zip(ks, ks[1:]):
            if t <= t1:
                e = ease((t - t0) / (t1 - t0)) if t1 > t0 else 1.0
                return z0 * (z1 / z0) ** e, x0 + (x1 - x0) * e, y0 + (y1 - y0) * e
        return ks[-1][1:]


def shoot(raw, start, end, cam, out):
    """raw[start:end] through the camera, frame by frame at sub-pixel
    precision, so slow zooms never step or jitter."""
    W, H = design.W, design.H
    dec = subprocess.Popen([FF, "-loglevel", "error", "-ss", f"{start:.3f}", "-to", f"{end:.3f}", "-i", str(raw),
                            "-r", "30", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], stdout=subprocess.PIPE)
    enc = subprocess.Popen([FF, "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}",
                            "-r", "30", "-i", "-", "-c:v", "libx264", "-preset", "medium", "-crf", "12",
                            "-pix_fmt", "yuv420p", str(out)], stdin=subprocess.PIPE)
    size, k = W * H * 3, 0
    while True:
        buf = dec.stdout.read(size)
        if len(buf) < size:
            break
        z, cx, cy = cam.at(start + k / 30)
        if abs(z - 1) < 1e-4:
            enc.stdin.write(buf)
        else:
            hw, hh = W / (2 * z), H / (2 * z)
            cx, cy = min(max(cx, hw), W - hw), min(max(cy, hh), H - hh)
            im = Image.frombuffer("RGB", (W, H), buf, "raw", "RGB", 0, 1)
            im = im.transform((W, H), Image.AFFINE, (1 / z, 0, cx - hw, 0, 1 / z, cy - hh), resample=Image.BICUBIC)
            enc.stdin.write(im.tobytes())
        k += 1
    enc.stdin.close()
    if enc.wait() or dec.wait():
        raise SystemExit(f"camera pass failed for {raw}")
    return out


def segment(name, raw, caps, start, end, cam=None):
    """Cuts raw[start:end], through the camera if there is one, and lays the
    caption labels and chapter tags on it."""
    if cam:
        raw = shoot(raw, start, end, cam, work / f"{name}-cam.mp4")
        clip = ["-i", raw]
    else:
        clip = ["-ss", f"{start:.2f}", "-to", f"{end:.2f}", "-i", raw]
    inputs, chains, last = [], [], "0:v"
    k = 1
    # One chapter tag per chapter run.
    runs = []
    for c in caps:
        ch = c.get("chapter") or ""
        if runs and runs[-1][0] == ch:
            runs[-1][2] = c["end"]
        else:
            runs.append([ch, c["start"], c["end"]])
    for ch, a, b in runs:
        if not ch:
            continue
        png = work / f"{name}-chap-{number(ch)}.png"
        design.chapter(number(ch), DISPLAY.get(ch, ch), png)
        inputs += ["-i", png]
        a, b = max(a, start), min(b if b is not None else end, end)
        chains.append(f"[{last}][{k}:v]overlay=0:0:enable='between(t,{a - start:.2f},{b - start:.2f})'[v{k}]")
        last, k = f"v{k}", k + 1
    for i, c in enumerate(caps):
        a, b = max(float(c["start"]), start), min(float(c["end"] or end), end)
        if b - a < 0.3:
            continue
        png = work / f"{name}-cap-{i}.png"
        design.caption(c["text"], png)
        inputs += ["-i", png]
        chains.append(f"[{last}][{k}:v]overlay=0:0:enable='between(t,{a - start:.2f},{b - start:.2f})'[v{k}]")
        last, k = f"v{k}", k + 1
    graph = ";".join(chains) if chains else "[0:v]null[v0]"
    out_label = last if chains else "v0"
    seg = work / f"{name}.mp4"
    run(*clip, *inputs,
        "-filter_complex", graph, "-map", f"[{out_label}]", "-r", "30",
        "-c:v", "libx264", "-preset", "slow", "-crf", "17", "-pix_fmt", "yuv420p", seg)
    return seg


def still(name, png, seconds):
    seg = work / f"{name}.mp4"
    run("-loop", "1", "-t", f"{seconds}", "-i", png, "-vf", "fps=30,format=yuv420p",
        "-c:v", "libx264", "-preset", "slow", "-crf", "16", "-tune", "stillimage", seg)
    return seg


# The SmartScreen chapter: from the double-click to resting on "Run anyway".
ss = load(ss_dir / "smartscreen2-captions.json")
for c in ss:
    c["chapter"] = "Download"
cl = load(cloud_dir / "setup-captions.json")

# The title lists only the chapters this cut shows, so a recording without
# the pairing chapter never promises one.
steps = []
for c in ss + cl:
    name = DISPLAY.get(c.get("chapter") or "", c.get("chapter") or "")
    if name and name not in steps:
        steps.append(name)
design.title_card(work / "title.png", steps)
design.end_card(work / "end.png")
parts = [still("title", work / "title.png", 3.6)]
ss_end = min(float(ss[-1]["end"]), duration(ss_dir / "smartscreen2-raw.mp4") - 0.1)


def cap(caps, prefix):
    return next(float(c["start"]) for c in caps if c["text"].startswith(prefix))


# Where things sit in the recordings (they're laid out by the record scripts):
# SmartScreen centred; the installer centred on the cloud screen; the agent
# centred 60px from the top, or left of Edge and 96px down when pairing.
mid = (design.W / 2, design.H / 2)
warn = cap(ss, "Windows may warn")
ss_cam = Camera([(0, 1.0, *mid), (warn + 0.15, 1.0, *mid), (warn + 1.15, 1.35, 720, 448)])
parts.append(segment("smartscreen", ss_dir / "smartscreen2-raw.mp4", ss, 0.6, ss_end, ss_cam))

# Hand edits for a recording, from edits.json beside its captions: caption
# starts to move, and stretches to drop (static waits only, so the cut can't
# be seen).
ef = cloud_dir / "edits.json"
edits = json.loads(ef.read_text(encoding="utf-8")) if ef.exists() else {}
for prefix, start in edits.get("retime", {}).items():
    i = next(i for i, c in enumerate(cl) if c["text"].startswith(prefix))
    cl[i]["start"] = start
    if i > 0:
        cl[i - 1]["end"] = start

# The cloud setup: everything after "Run anyway".
cl[0]["text"] = "After Run anyway, the installer opens"
cl_end = min(float(cl[-1]["end"]), duration(cloud_dir / "setup-raw.mp4") - 0.1)
# Skip the seconds where the agent sits loading after Finish: cut from a few
# seconds after the Finish caption to just before the next one.
cuts = [(0.6, cl_end)]
fin = next((i for i, c in enumerate(cl) if c["text"].startswith("Click Finish")), None)
if fin is not None and fin + 1 < len(cl):
    a, b = float(cl[fin]["start"]) + 4.6, float(cl[fin + 1]["start"]) - 0.8
    if b - a > 1.5:
        cuts = [(0.6, a), (b, cl_end)]
for d0, d1 in edits.get("drop", []):
    cuts = [p for s, e in cuts for p in ([(s, d0), (d1, e)] if s < d0 < d1 < e else [(s, e)])]
pairing = any(c.get("chapter") == "Pair" for c in cl)
agent_x, agent_top = (340, 96) if pairing else (720, 60)
finish, find, close = cap(cl, "Click Finish"), cap(cl, "It finds your card printer"), cap(cl, "Close the window")
# Close on the installer; back out while the agent starts (that stretch is
# trimmed); push in on the agent's Badge printer card for choosing the
# printer and the test card; out to the whole window to close it.
near = (1.6, agent_x, agent_top + 480)
if pairing:
    # Pairing, with the agent on the left and Edge on the right: in on the
    # agent's status card, across to the pairing page (its prompt to reach
    # the agent shows over the same view), back to the agent once it's ready.
    pair, web, back = cap(cl, "Pair it"), cap(cl, "BadgeBadger opens"), cap(cl, "Back in the agent")
    on_agent, on_page = (1.7, 400, 300), (1.55, 960, 443)
    keys = [(pair - 0.8, 1.0, *mid), (pair + 0.2, 1.0, *mid), (pair + 1.4, *on_agent), (web + 0.2, *on_agent),
            (web + 1.4, *on_page), (back - 0.9, *on_page), (back + 0.3, *on_agent), (find + 0.2, *on_agent)]
else:
    keys = [(find - 0.8, 1.0, *mid), (find + 0.3, 1.0, *mid)]
cloud_cam = Camera([(0, 1.45, 718, 485), (finish + 4.6, 1.45, 718, 485), *keys,
                    (find + 1.3, *near), (close - 0.9, *near), (close + 0.1, 1.0, *mid)])
for n, (a, b) in enumerate(cuts):
    parts.append(segment(f"setup{n}", cloud_dir / "setup-raw.mp4", cl, a, b, cloud_cam))
parts.append(still("end", work / "end.png", 4.2))

# Crossfade the parts together.
durs = [duration(p) for p in parts]
args, graph, prev, offset = [], [], "0:v", 0.0
for p in parts:
    args += ["-i", p]
for i in range(1, len(parts)):
    offset += durs[i - 1] - FADE
    label = f"x{i}"
    graph.append(f"[{prev}][{i}:v]xfade=transition=fade:duration={FADE}:offset={offset:.2f}[{label}]")
    prev = label
run(*args, "-filter_complex", ";".join(graph), "-map", f"[{prev}]",
    "-c:v", "libx264", "-preset", "slow", "-crf", "20", "-pix_fmt", "yuv420p", "-movflags", "+faststart", out)
# Poster: the title card, for the page before the video plays.
run("-i", work / "title.png", "-q:v", "2", out.with_suffix(".jpg"))
print("wrote", out, f"{duration(out):.1f}s", out.stat().st_size, "bytes")
