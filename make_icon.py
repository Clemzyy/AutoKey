"""Génère l'icône d'AutoKey (assets/autokey.ico + assets/autokey.png). Nécessite Pillow."""
import math
import os

from PIL import Image, ImageDraw, ImageFilter

S = 1024            # dessin en grand, réduit ensuite pour un rendu net
HERE = os.path.dirname(os.path.abspath(__file__))


def gradient(size, c1, c2):
    """Dégradé diagonal c1 (haut-gauche) -> c2 (bas-droite)."""
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * size - 2)
            px[x, y] = tuple(int(a + (b - a) * t) for a, b in zip(c1, c2))
    return img


def rounded_mask(size, box, radius):
    m = Image.new("L", (size, size), 0)
    ImageDraw.Draw(m).rounded_rectangle(box, radius, fill=255)
    return m


def build():
    base = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    # fond : carré arrondi dégradé violet -> cyan
    bg = gradient(S, (108, 70, 255), (0, 200, 255)).convert("RGBA")
    base.paste(bg, (0, 0), rounded_mask(S, (24, 24, S - 24, S - 24), 230))

    # ombre portée de la touche
    shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle((170, 215, 854, 899), 130, fill=(10, 8, 60, 150))
    base.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(28)))

    d = ImageDraw.Draw(base)
    # touche de clavier : socle (bord) puis dessus clair
    d.rounded_rectangle((165, 190, 859, 864), 130, fill=(196, 188, 255))
    d.rounded_rectangle((165, 170, 859, 800), 130, fill=(255, 255, 255))
    d.rounded_rectangle((225, 225, 799, 745), 85, fill=(244, 242, 255))

    # horloge au centre de la touche
    cx, cy, r = 512, 485, 190
    d.ellipse((cx - r, cy - r, cx + r, cy + r), outline=(92, 60, 235), width=46)
    for i in range(12):                                        # graduations
        a = math.radians(i * 30)
        r1, r2 = r - 70, r - 105
        d.line((cx + r1 * math.sin(a), cy - r1 * math.cos(a), cx + r2 * math.sin(a), cy - r2 * math.cos(a)),
               fill=(92, 60, 235), width=14)
    d.line((cx, cy, cx, cy - 115), fill=(92, 60, 235), width=40)                       # grande aiguille
    d.line((cx, cy, cx + 85, cy + 45), fill=(0, 190, 245), width=40)                   # petite aiguille
    d.ellipse((cx - 36, cy - 36, cx + 36, cy + 36), fill=(92, 60, 235))
    for dx, dy in ((0, -115), (85, 45)):                       # bouts arrondis
        d.ellipse((cx + dx - 20, cy + dy - 20, cx + dx + 20, cy + dy + 20),
                  fill=(92, 60, 235) if dx == 0 else (0, 190, 245))
    return base


if __name__ == "__main__":
    out = os.path.join(HERE, "assets")
    os.makedirs(out, exist_ok=True)
    icon = build()
    icon.resize((512, 512), Image.LANCZOS).save(os.path.join(out, "autokey.png"))
    icon.save(os.path.join(out, "autokey.ico"), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64),
                                                       (128, 128), (256, 256)])
    print("icône générée dans", out)
