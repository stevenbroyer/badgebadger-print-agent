# The designed pieces of the setup video: title and end cards (a badge on a
# lanyard, like the agent's own status card), numbered chapter tags and
# caption labels, all in the agent's colours and the web app's fonts.
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H = 1440, 900
# Everything was laid out at 1280x800; S scales it to the frame.
S = W / 1280


def px(v):
    return int(round(v * S))
HERE = Path(__file__).parent
FONTS = HERE / "fonts"
ICON = HERE.parents[1] / "src-tauri" / "icons" / "128x128@2x.png"

PAPER = (236, 233, 226)
CARD = (251, 250, 247)
INK = (26, 27, 32)
INK2 = (70, 75, 85)
INK3 = (95, 99, 108)
LANYARD = (30, 136, 229)
PRESS = (21, 101, 192)
READY = (47, 125, 85)


def font(name, size, axes):
    f = ImageFont.truetype(str(FONTS / name), size)
    f.set_variation_by_axes(axes)
    return f


def outfit(size, weight=600):
    return font("Outfit.ttf", size, [weight])


def dm(size, weight=450):
    return font("DMSans.ttf", size, [min(max(size, 9), 40), weight])


def mono(size):
    return ImageFont.truetype(r"C:\Windows\Fonts\CascadiaMono.ttf" if Path(r"C:\Windows\Fonts\CascadiaMono.ttf").exists() else r"C:\Windows\Fonts\consola.ttf", size)


def shadow(size, box, radius, blur, alpha):
    layer = Image.new("RGBA", size, (0, 0, 0, 0))
    ImageDraw.Draw(layer).rounded_rectangle(box, radius, fill=(26, 27, 32, alpha))
    return layer.filter(ImageFilter.GaussianBlur(blur))


def hairline(canvas, box, radius, alpha=28):
    """A soft 1px edge, composited so it stays a hairline."""
    layer = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    ImageDraw.Draw(layer).rounded_rectangle(box, radius, outline=(26, 27, 32, alpha), width=1)
    canvas.alpha_composite(layer)


def badge(canvas, top, band, lines):
    """A badge hanging from a lanyard strap, centred, with the badger photo."""
    d = ImageDraw.Draw(canvas)
    cw, ch = px(560), px(300)
    x0 = (W - cw) // 2
    y0 = top
    r = px(22)
    # Strap from the top edge down to the clip.
    d.rounded_rectangle([W // 2 - px(8), -px(10), W // 2 + px(8), y0 + px(6)], px(3), fill=PRESS)
    canvas.alpha_composite(shadow((W, H), [x0, y0 + px(10), x0 + cw, y0 + ch + px(10)], r, px(18), 60))
    d.rounded_rectangle([x0, y0, x0 + cw, y0 + ch], r, fill=CARD)
    # Band in the state colour with a firm edge, clipped to the card's shape.
    card_mask = Image.new("L", (W, H), 0)
    ImageDraw.Draw(card_mask).rounded_rectangle([x0, y0, x0 + cw, y0 + ch], r, fill=255)
    band_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    bd = ImageDraw.Draw(band_layer)
    bd.rectangle([x0, y0, x0 + cw, y0 + px(44)], fill=band + (38,))
    bd.rectangle([x0, y0 + px(44), x0 + cw, y0 + px(46)], fill=band + (255,))
    band_layer.putalpha(Image.composite(band_layer.getchannel("A"), Image.new("L", (W, H), 0), card_mask))
    canvas.alpha_composite(band_layer)
    hairline(canvas, [x0, y0, x0 + cw, y0 + ch], r)
    d.rounded_rectangle([W // 2 - px(30), y0 + px(16), W // 2 + px(30), y0 + px(26)], px(5), fill=PAPER)
    # The clip over the punch.
    clip = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    cd = ImageDraw.Draw(clip)
    cd.rounded_rectangle([W // 2 - px(22), y0 + px(2), W // 2 + px(22), y0 + px(20)], px(4), fill=(186, 192, 201))
    cd.rectangle([W // 2 - px(20), y0 + px(4), W // 2 + px(20), y0 + px(9)], fill=(220, 224, 230))
    canvas.alpha_composite(clip)
    # Photo: the badger on the link-preview badge's backdrop.
    fx, fy, fw, fh = x0 + px(36), y0 + px(76), px(150), px(180)
    photo = Image.new("RGBA", (fw, fh))
    for yy in range(fh):
        t = yy / fh
        c = tuple(int(a + (b - a) * t) for a, b in zip((237, 242, 248), (211, 221, 233)))
        ImageDraw.Draw(photo).line([(0, yy), (fw, yy)], fill=c + (255,))
    mask = Image.new("L", (fw, fh), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, fw - 1, fh - 1], px(14), fill=255)
    size = px(132)
    icon = Image.open(ICON).convert("RGBA").resize((size, size), Image.LANCZOS)
    photo.alpha_composite(icon, ((fw - size) // 2, fh - px(140)))
    canvas.paste(photo, (fx, fy), mask)
    hairline(canvas, [fx, fy, fx + fw, fy + fh], px(14))
    d.ellipse([fx + fw - px(22), fy + px(10), fx + fw - px(10), fy + px(22)], fill=band, outline=CARD, width=2)
    # Lines of text beside the photo.
    tx, ty = fx + fw + px(30), fy + px(14)
    for text, f, color, gap in lines:
        d.text((tx, ty), text, font=f, fill=color)
        ty += f.size + gap
    return canvas


def title_card(path, steps=("Download", "Install", "Pair", "Choose your printer")):
    c = Image.new("RGBA", (W, H), PAPER + (255,))
    badge(c, px(196), READY, [
        ("Setting up", dm(px(20), 500), INK3, px(6)),
        ("BadgeBadger", outfit(px(40), 650), INK, px(2)),
        ("Print Agent", outfit(px(40), 450), INK2, px(18)),
        ("Windows · about two minutes", dm(px(19), 450), INK3, 0),
    ])
    d = ImageDraw.Draw(c)
    line = "   ·   ".join(steps)
    f = dm(px(21), 500)
    d.text(((W - d.textlength(line, font=f)) / 2, px(588)), line, font=f, fill=INK2)
    c.convert("RGB").save(path)


def end_card(path):
    c = Image.new("RGBA", (W, H), PAPER + (255,))
    badge(c, px(196), READY, [
        ("Ready to print", outfit(px(40), 650), INK, px(14)),
        ("Print a badge in BadgeBadger", dm(px(21), 450), INK2, px(4)),
        ("and it comes out of your", dm(px(21), 450), INK2, px(4)),
        ("card printer.", dm(px(21), 450), INK2, 0),
    ])
    d = ImageDraw.Draw(c)
    f = dm(px(20), 500)
    help_text = "Help and troubleshooting: badgebadger.app/docs"
    d.text(((W - d.textlength(help_text, font=f)) / 2, px(588)), help_text, font=f, fill=INK3)
    c.convert("RGB").save(path)


def caption(text, path):
    """A rounded label low in the frame, sized to its text."""
    c = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    f = dm(px(25), 520)
    d = ImageDraw.Draw(c)
    tw = d.textlength(text, font=f)
    pw, ph = int(tw + px(56)), px(58)
    x, y = (W - pw) // 2, H - px(40) - ph
    c.alpha_composite(shadow((W, H), [x, y + px(6), x + pw, y + ph + px(6)], px(18), px(12), 70))
    d.rounded_rectangle([x, y, x + pw, y + ph], px(18), fill=(20, 22, 27, 236))
    d.text((x + px(28), y + (ph - f.size) / 2 - px(3)), text, font=f, fill=(250, 248, 243))
    c.save(path)


def chapter(n, name, path):
    """Top-left tag: a numbered coin and the chapter's name."""
    c = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(c)
    f = outfit(px(19), 600)
    th = px(46)
    w = int(px(56) + d.textlength(name, font=f) + px(20))
    x, y = px(28), px(24)
    c.alpha_composite(shadow((W, H), [x, y + px(4), x + w, y + th + px(2)], px(16), px(10), 45))
    d.rounded_rectangle([x, y, x + w, y + th], px(16), fill=CARD + (250,))
    hairline(c, [x, y, x + w, y + th], px(16), 34)
    d.ellipse([x + px(9), y + px(8), x + px(39), y + px(38)], fill=LANYARD)
    num = outfit(px(17), 650)
    d.text((x + px(24) - d.textlength(str(n), font=num) / 2, y + px(12)), str(n), font=num, fill=(255, 255, 255))
    d.text((x + px(50), y + px(11)), name, font=f, fill=INK)
    c.save(path)


if __name__ == "__main__":
    out = HERE / "preview"
    out.mkdir(exist_ok=True)
    title_card(out / "title.png")
    end_card(out / "end.png")
    caption("Click More info: it names the verified publisher, Steven Broyer", out / "cap.png")
    chapter(4, "Choose your printer", out / "chap.png")
    # Overlays on a sample frame to judge them in place.
    frame = Image.new("RGBA", (W, H), (214, 222, 232, 255))
    for p in ("chap.png", "cap.png"):
        frame.alpha_composite(Image.open(out / p))
    frame.convert("RGB").save(out / "overlay-sample.png")
    print("ok")
