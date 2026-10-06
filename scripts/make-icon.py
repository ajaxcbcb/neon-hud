"""Generate the original Neon HUD app mark. Requires Pillow."""
from pathlib import Path
from PIL import Image, ImageDraw

target = Path(__file__).resolve().parents[1] / 'src-tauri' / 'icons'
target.mkdir(parents=True, exist_ok=True)
image = Image.new('RGBA', (1024, 1024), (8, 18, 23, 255))
draw = ImageDraw.Draw(image)
draw.rounded_rectangle((54, 54, 970, 970), radius=190, outline=(67, 229, 241), width=35)
draw.line([(280, 742), (280, 284), (744, 742), (744, 284)], fill=(67, 229, 241), width=85)
draw.line([(302, 748), (738, 748)], fill=(154, 245, 191), width=12)
image.save(target / 'icon.png')
for size in (32, 128):
    image.resize((size, size), Image.Resampling.LANCZOS).save(target / f'{size}x{size}.png')
image.resize((256, 256), Image.Resampling.LANCZOS).save(target / '128x128@2x.png')
image.save(target / 'icon.ico', sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
image.save(target / 'icon.icns')
