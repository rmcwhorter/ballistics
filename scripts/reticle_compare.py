"""Gen 1 (Raptor M8 Yard) vs Gen 2 (Raptor 5.56/.308 Yard G2) side-by-side image.

    uv run --with numpy --with pillow scripts/reticle_compare.py

Uses Primary Arms' raster renders from their PLx reticle guide (downloaded to data/).
The two renders are drawn at different scales; both are resampled to the same px/MIL:
  - M8: 10.65 px/MIL, from the 1-MIL hash spacing on the vertical line and the I-bars at
    10 and 20 MIL (106.5 / 213.5 px from center at (499.5, 374.5)).
  - G2: 11.53 px/MIL, from the 10/15/20 MIL marks on the thick posts (115.5 / 173 / 231 px
    from center at (499.5, 362.0)).
Cross-check: after scaling, the 400-800 yd BDC rows agree between the two to ~0.05 MIL.
MIL values in the "What changed" notes are pixel measurements off these renders (except the
BDC rows, which match the vector extraction in extract_reticle.py). Needs macOS Helvetica
and ImageMagick (`magick`) to decode AVIF.
"""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA, OUT = ROOT / "data", ROOT / "out"
for name in ["ACSS-Raptor-M8-Yards", "RAPTOR-5.56Y-G2-1"]:
    png = DATA / f"{name}.png"
    if not png.exists():
        DATA.mkdir(exist_ok=True)
        avif = DATA / f"{name}.avif"
        subprocess.run(["curl", "-sSL", "-A", "Mozilla/5.0", "-o", str(avif),
                        f"https://blog.primaryarms.com/wp-content/uploads/{name}.avif"], check=True)
        subprocess.run(["magick", str(avif), str(png)], check=True)

from PIL import Image, ImageDraw, ImageFont, ImageChops
import numpy as np

SRC = {
  'm8': dict(f=DATA / 'ACSS-Raptor-M8-Yards.png', pxmil=10.65, c=(499.5,374.5)),
  'g2': dict(f=DATA / 'RAPTOR-5.56Y-G2-1.png',     pxmil=11.53, c=(499.5,362.0)),
}
def load(k, S):
    d=SRC[k]; im=Image.open(d['f']).convert('RGBA')
    bg=Image.new('RGBA',im.size,(255,255,255,255)); bg.alpha_composite(im); im=bg.convert('RGB')
    f=S/d['pxmil']; im=im.resize((round(im.width*f),round(im.height*f)),Image.LANCZOS)
    return im,(d['c'][0]*f,d['c'][1]*f)
def crop(k,S,x0,x1,y0,y1):  # mil window, +y down
    im,(cx,cy)=load(k,S)
    box=(round(cx+x0*S),round(cy+y0*S),round(cx+x1*S),round(cy+y1*S))
    out=Image.new('RGB',(box[2]-box[0],box[3]-box[1]),'white'); out.paste(im.crop(box))
    return out
def tint(im,rgb):
    a=np.asarray(im).astype(float)/255; ink=1-a.min(2)  # darkness
    col=np.array(rgb,float)/255
    out=1-ink[...,None]*(1-col)
    return Image.fromarray((out*255).astype(np.uint8))
def overlay(a,b):
    return ImageChops.multiply(tint(a,(37,99,235)),tint(b,(234,88,12)))

F='/System/Library/Fonts/Helvetica.ttc'
def font(sz,bold=False): return ImageFont.truetype(F,sz,index=1 if bold else 0)
INK=(20,20,24); MUTED=(95,95,105); BLUE=(37,99,235); ORANGE=(234,88,12)

S=14; R=25.2   # full view: +/-25.2 MIL
full={k:crop(k,S,-R,R,-R,R) for k in SRC}; full['ov']=overlay(full['m8'],full['g2'])
Z=27.5; zx=(-12.5,12.5); zy=(-6.5,11.0)
zoom={k:crop(k,Z,zx[0],zx[1],zy[0],zy[1]) for k in SRC}; zoom['ov']=overlay(zoom['m8'],zoom['g2'])

pw=full['m8'].width; gap=40; M=60
zw,zh=zoom['m8'].size
W=M*2+pw*3+gap*2
notes=[
 ("Illuminated horseshoe", "Gen 1: big ~11.3 MIL horseshoe, ~1.4 MIL thick stroke.  G2: tiny ~2.4 MIL horseshoe + micro chevron (diffractive 'Red Dot Bright' element)."),
 ("1x / CQB aiming aid", "Gen 1 relies on the big etched horseshoe.  G2 swaps that for heavy black posts at 3, 6 and 9 o'clock and a bright red-dot-sized horseshoe."),
 ("12 o'clock", "Gen 1 has a full hashed 1 MIL crosshair to the top of the FOV.  G2 has no upper stadia at all - open top."),
 ("BDC drops", "Unchanged: 400-800 yd rows at ~1.7 / 2.8 / 4.1 / 5.6 / 7.4 MIL in both (BDC still ends at 800). The '10' / '20' on G2's bottom post are MIL marks."),
 ("Wind / lead dots", "Gen 1 BDC wind dots are tighter (~1.3 MIL at 800).  G2's are wider (~1.6 MIL at 800) and add 3 moving-target lead dots each side (3 / 6 / 9 mph at 1.7 / 3.3 / 5.0 MIL)."),
 ("Ranging stadia", "Gen 1: 300-800 yd vertical bars spanning ~7-17 MIL out on the horizontal.  G2: 400-800 yd 5'10\" height bars pulled in to ~6-11 MIL, plus a 1 MIL scale."),
 ("Outer crosshair", "Gen 1: thin line, 1 MIL hashes, I-bars every 10 MIL to the edge.  G2: thick posts from ~12 MIL (sides) / ~8.5 MIL (bottom) with marks at 10, 15, 20 MIL."),
]
nf=font(25,True); nf2=font(24); lh=40
H=M+150+44+pw+70+44+zh+60+ 30+len(notes)*lh+60+50
img=Image.new('RGB',(W,H),(250,250,248)); d=ImageDraw.Draw(img)
y=M
d.text((M,y),"Primary Arms PLxC 1-8x24 FFP  -  ACSS Raptor 5.56/.308 Yard BDC reticle",font=font(46,True),fill=INK); y+=62
d.text((M,y),"Gen 1 (ACSS Raptor M8 Yard, etched)   vs.   Gen 2 'RDB' (ACSS Raptor 5.56/.308 Yard G2, diffractive)",font=font(30),fill=MUTED); y+=44
d.text((M,y),"Both shown at 8x and at the SAME MIL scale (FFP, so subtensions are identical at every power). Scaled from Primary Arms' official reticle renders using the 10/20 MIL markers.",font=font(22),fill=MUTED); y+=44
heads=[("GEN 1  -  Raptor M8 Yard",INK),("GEN 2  -  Raptor Yard G2 RDB",INK),("OVERLAY",INK)]
def panels(row, y, sub):
    for i,k in enumerate(['m8','g2','ov']):
        x=M+i*(pw+gap)
        if sub:
            d.text((x,y),heads[i][0]+sub,font=font(28,True),fill=INK)
            if k=='ov':
                tx=x+d.textlength(heads[i][0]+sub+"   ",font=font(28,True))
                d.text((tx,y+2),"Gen 1",font=font(26,True),fill=BLUE); tx+=d.textlength("Gen 1  ",font=font(26,True))
                d.text((tx,y+2),"Gen 2",font=font(26,True),fill=ORANGE)
        im=row[k]; ox=x+(pw-im.width)//2
        img.paste(im,(ox,y+44)); d.rectangle((ox-1,y+43,ox+im.width,y+44+im.height),outline=(210,210,210))
panels(full,y,""); y+=44+pw+70
d.text((M,y-36),"Center detail (zoomed, still same scale between panels)",font=font(24),fill=MUTED)
zoom2={k:zoom[k] for k in zoom}
# center zoom panels in same columns
for i,k in enumerate(['m8','g2','ov']):
    x=M+i*(pw+gap); d.text((x,y),["GEN 1","GEN 2 RDB","OVERLAY"][i]+"  -  center",font=font(28,True),fill=INK)
    ox=x+(pw-zw)//2; img.paste(zoom[k],(ox,y+44)); d.rectangle((ox-1,y+43,ox+zw,y+44+zh),outline=(210,210,210))
y+=44+zh+60
d.text((M,y),"What changed",font=font(34,True),fill=INK); y+=54
for h,t in notes:
    d.text((M,y),h,font=nf,fill=INK); d.text((M+330,y),t,font=nf2,fill=(50,50,58)); y+=lh
y+=20
d.text((M,y),"MIL values are approximate, measured off the manufacturer renders (not the printed manuals). Gen 1 render: blog.primaryarms.com ACSS-Raptor-M8-Yards; Gen 2: RAPTOR-5.56Y-G2.",font=font(20),fill=MUTED)
img=img.crop((0,0,W,y+50))
OUT.mkdir(exist_ok=True)
img.save(OUT / 'plxc_raptor_gen1_vs_g2.png'); print(OUT / 'plxc_raptor_gen1_vs_g2.png', img.size)
