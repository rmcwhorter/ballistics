"""Reticle comparison image: PLxC Gen 1 (Raptor M8 Yard) vs Gen 2 (Raptor 5.56/.308 Yard G2),
and Gen 2 vs the Trijicon Credo 1-8x28 FFP MRAD Segmented Circle (CR828-C-2900032).

    uv run --with numpy --with pillow scripts/reticle_compare.py

Uses the manufacturers' raster renders (downloaded to data/). They are drawn at different
scales; each is resampled to a common px/MIL (MIL = MRAD):
  - M8: 10.65 px/MIL, from the 1-MIL hash spacing on the vertical line and the I-bars at
    10 and 20 MIL (106.5 / 213.5 px from center at (499.5, 374.5)).
  - G2: 11.53 px/MIL, from the 10/15/20 MIL marks on the thick posts (115.5 / 173 / 231 px
    from center at (499.5, 362.0)).
  - Credo: 11.92 px/MRAD, from the 5/10/15/20/25 MRAD long hashes on the horizontal
    (60 / 119 / 179 / 238.5 / 298 px from center at (801.5, 799.5)). Trijicon's dimension
    sheet (Credo_MRAD_Segmented_Circle_FFP_Reticle_2900032.pdf) says "not drawn to scale",
    so its numbers are used as a cross-check instead: `credo_check()` measures the ring
    (19.3 MRAD across the segment ends, 7.5 MRAD gaps, 1.5 MRAD thick) off the render.
Cross-check: after scaling, the 400-800 yd BDC rows agree between M8 and G2 to ~0.05 MIL.
MIL values in the notes are pixel measurements off these renders, except the G2 BDC rows,
wind holds, lead dots, horseshoe and ranging-bar positions, which come from the vector
extraction in extract_reticle.py, and the Credo values Trijicon's sheet states. Needs macOS
Helvetica and ImageMagick (`magick`) to decode AVIF.
"""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA, OUT = ROOT / "data", ROOT / "out"
UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126 Safari/537.36"
for name in ["ACSS-Raptor-M8-Yards", "RAPTOR-5.56Y-G2-1"]:
    png = DATA / f"{name}.png"
    if not png.exists():
        DATA.mkdir(exist_ok=True)
        avif = DATA / f"{name}.avif"
        subprocess.run(["curl", "-sSL", "-A", "Mozilla/5.0", "-o", str(avif),
                        f"https://blog.primaryarms.com/wp-content/uploads/{name}.avif"], check=True)
        subprocess.run(["magick", str(avif), str(png)], check=True)
CREDO = DATA / "CR828-C-2900032-reticle.png"
if not CREDO.exists():
    DATA.mkdir(exist_ok=True)
    subprocess.run(["curl", "-sSL", "-A", UA, "-o", str(CREDO),
                    "https://www.trijicon.com/uploads/product-uploads/reticle-images/CR828-C-2900032-reticle.png"], check=True)

from PIL import Image, ImageDraw, ImageFont, ImageChops
import numpy as np

SRC = {
  'm8': dict(f=DATA / 'ACSS-Raptor-M8-Yards.png', pxmil=10.65, c=(499.5,374.5)),
  'g2': dict(f=DATA / 'RAPTOR-5.56Y-G2-1.png',     pxmil=11.53, c=(499.5,362.0)),
  'credo': dict(f=CREDO,                           pxmil=11.92, c=(801.5,799.5)),
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
def overlay(a,b,ca,cb):
    return ImageChops.multiply(tint(a,ca),tint(b,cb))

def credo_check():
    """Measure the Credo ring off the render at its px/MRAD and compare with Trijicon's sheet."""
    d=SRC['credo']; a=np.asarray(Image.open(d['f']).convert('RGB')).astype(int)
    red=(a[...,0]>180)&(a[...,1]<120)&(a[...,2]<100)
    ys,xs=np.nonzero(red); cx,cy=d['c']; k=d['pxmil']
    dx,dy=(xs-cx)/k,(ys-cy)/k; ring=np.hypot(dx,dy)>2
    side=ring&(np.abs(dy)<4.5)  # pixels of the side segments next to their ends
    across=dx[side].max()-dx[side].min()
    gap=2*np.abs(dx[ring&(dy<-5)]).min()
    print(f"Credo ring at {k} px/MRAD: across segment ends {across:.2f} (sheet 19.3), gap {gap:.2f} (7.5); +/-0.08 per px")
    assert abs(across-19.3)<0.16 and abs(gap-7.5)<0.16, "Credo scale disagrees with Trijicon's sheet"
credo_check()

F='/System/Library/Fonts/Helvetica.ttc'
def font(sz,bold=False): return ImageFont.truetype(F,sz,index=1 if bold else 0)
INK=(20,20,24); MUTED=(95,95,105); BLUE=(37,99,235); ORANGE=(234,88,12); GREEN=(22,140,70)

PW=706; gap=40; M=60
W=M*2+PW*3+gap*2
NOTES_A=[
 ("Illuminated horseshoe", "Gen 1: big ~11.3 MIL horseshoe, ~1.4 MIL thick stroke.  G2: tiny 2.5 MIL horseshoe + micro chevron (diffractive 'Red Dot Bright' element)."),
 ("1x / CQB aiming aid", "Gen 1 relies on the big etched horseshoe.  G2 swaps that for heavy black posts at 3, 6 and 9 o'clock and a bright red-dot-sized horseshoe."),
 ("12 o'clock", "Gen 1 has a full hashed 1 MIL crosshair to the top of the FOV.  G2 has no upper stadia at all - open top."),
 ("BDC drops", "Unchanged: 400-800 yd rows at ~1.7 / 2.8 / 4.1 / 5.6 / 7.4 MIL in both (BDC still ends at 800). The '10' / '20' on G2's bottom post are MIL marks."),
 ("Wind / lead dots", "Gen 1 BDC wind dots are tighter (~1.3 MIL at 800).  G2: 5 / 10 mph dots (1.57 / 3.14 MIL at 800), and the row numerals 4 / 6 / 8 sit at the 15 mph hold."),
 ("", "G2 also adds 3 moving-target lead dots each side of the horseshoe (3 / 6 / 9 mph at 1.70 / 3.30 / 5.00 MIL)."),
 ("Ranging stadia", "Gen 1: 300-800 yd vertical bars spanning ~7-17 MIL out on the horizontal.  G2: 400-800 yd 5'10\" height bars at exactly 6-10 MIL, plus a 1 MIL tick scale."),
 ("Outer crosshair", "Gen 1: thin line, 1 MIL hashes, I-bars every 10 MIL to the edge.  G2: thick posts from ~12 MIL (sides) / ~8.5 MIL (bottom) with marks at 10, 15, 20 MIL."),
]
NOTES_B=[
 ("Units", "Both are first focal plane and milliradian (MIL = MRAD), so this overlay holds at every magnification."),
 ("1x / CQB aiming aid", "G2: 2.5 MIL red horseshoe + chevron, plus heavy posts from ~12 MIL.  Credo: 0.15 MRAD dot in a 1 MRAD red cross, inside a big"),
 ("", "segmented red ring (~20.7 MRAD outer diameter, 19.3 across the segment ends, 1.5 thick, 7.5 MRAD gaps at 12 / 3 / 6 / 9)."),
 ("Elevation", "G2: BDC rows labeled by range, 300-800 yd at 0.93-7.36 MIL.  Credo: no BDC, a plain mil ladder: 0.5 MRAD hashes from 1-5 MRAD,"),
 ("", "1 MRAD hashes out to 29, numbered every 5 below center. The G2's 400 / 500 / 600 / 800 rows are 1.74 / 2.81 / 4.12 / 7.36 MRAD holds on it."),
 ("Wind", "G2: 5 / 10 / 15 mph holds per row (true for a ~77gr at 2600-2650 fps).  Credo: the same 0.5 / 1 MRAD hashes on the horizontal;"),
 ("", "you hold mils, which works for any load once you know its drift (77gr @ 2600: 1.66 MRAD per 10 mph at 500 yd)."),
 ("Ranging / leads", "G2: 5'10\" and 18\" auto-ranging bars and 3 / 6 / 9 mph lead dots.  Credo: mil-relation only (yards = inches x 27.8 / MRAD), no lead dots."),
 ("Outer crosshair", "G2: thick posts from ~12 MIL.  Credo: thick posts from 30 MRAD, outside the 8x field of view (2.53 deg = +/-22 MRAD), so at 8x it is"),
 ("", "an open hashed crosshair; the posts come into view below roughly 6x."),
]
nf=font(25,True); nf2=font(24); lh=40

def section(keys, names, colors, R, zx, zy):
    """Full-view row and center-zoom row for two reticles plus their overlay."""
    S=PW/(2*R); Z=27.5
    full={k:crop(k,S,-R,R,-R,R) for k in keys}; full['ov']=overlay(full[keys[0]],full[keys[1]],*colors)
    zoom={k:crop(k,Z,*zx,*zy) for k in keys}; zoom['ov']=overlay(zoom[keys[0]],zoom[keys[1]],*colors)
    return S,full,zoom

def fov_circle(im, S, r_mil, label):
    dd=ImageDraw.Draw(im); cx,cy=im.width/2,im.height/2; rp=r_mil*S
    for a in range(0,360,8):
        dd.arc((cx-rp,cy-rp,cx+rp,cy+rp),a,a+4,fill=(120,120,130),width=2)
    f=font(18); dd.text((12,im.height-30),"- - -  "+label,font=f,fill=MUTED)

def draw_section(d, img, y, keys, titles, names, colors, R, zx, zy, fov=None):
    S,full,zoom=section(keys, names, colors, R, zx, zy)
    if fov:
        for k in (keys[1],'ov'): fov_circle(full[k],S,*fov)
    order=[keys[0],keys[1],'ov']
    for i,k in enumerate(order):
        x=M+i*(PW+gap); d.text((x,y),titles[i],font=font(28,True),fill=INK)
        if k=='ov':
            tx=x+d.textlength(titles[i]+"   ",font=font(28,True))
            d.text((tx,y+2),names[0],font=font(26,True),fill=colors[0]); tx+=d.textlength(names[0]+"  ",font=font(26,True))
            d.text((tx,y+2),names[1],font=font(26,True),fill=colors[1])
        im=full[k]; ox=x+(PW-im.width)//2
        img.paste(im,(ox,y+44)); d.rectangle((ox-1,y+43,ox+im.width,y+44+im.height),outline=(210,210,210))
    y+=44+PW+70
    zw,zh=zoom[keys[0]].size
    d.text((M,y-36),f"Center detail (zoomed, same scale between panels; full view above is +/-{R:g} MIL)",font=font(24),fill=MUTED)
    for i,k in enumerate(order):
        x=M+i*(PW+gap); d.text((x,y),titles[i]+"  -  center",font=font(28,True),fill=INK)
        ox=x+(PW-zw)//2; img.paste(zoom[k],(ox,y+44)); d.rectangle((ox-1,y+43,ox+zw,y+44+zh),outline=(210,210,210))
    return y+44+zh+60

def draw_notes(d, y, title, notes):
    d.text((M,y),title,font=font(34,True),fill=INK); y+=54
    for h,t in notes:
        d.text((M,y),h,font=nf,fill=INK); d.text((M+330,y),t,font=nf2,fill=(50,50,58)); y+=lh
    return y

img=Image.new('RGB',(W,6000),(250,250,248)); d=ImageDraw.Draw(img)
y=M
d.text((M,y),"Primary Arms PLxC 1-8x24 FFP  -  ACSS Raptor 5.56/.308 Yard BDC reticle",font=font(46,True),fill=INK); y+=62
d.text((M,y),"Gen 1 (ACSS Raptor M8 Yard, etched)   vs.   Gen 2 'RDB' (ACSS Raptor 5.56/.308 Yard G2, diffractive)",font=font(30),fill=MUTED); y+=44
d.text((M,y),"Both shown at 8x and at the SAME MIL scale (FFP, so subtensions are identical at every power). Scaled from Primary Arms' official reticle renders using the 10/20 MIL markers.",font=font(22),fill=MUTED); y+=44
y=draw_section(d,img,y,['m8','g2'],["GEN 1  -  Raptor M8 Yard","GEN 2  -  Raptor Yard G2 RDB","OVERLAY"],
               ["Gen 1","Gen 2"],(BLUE,ORANGE),25.2,(-12.5,12.5),(-6.5,11.0))
y=draw_notes(d,y,"What changed",NOTES_A)

y+=50; d.line((M,y,W-M,y),fill=(205,205,200),width=2); y+=40
d.text((M,y),"PLxC Gen 2 (ACSS Raptor G2)   vs.   Trijicon Credo 1-8x28 FFP, MRAD Segmented Circle (CR828-C-2900032)",font=font(40,True),fill=INK); y+=58
d.text((M,y),"Same MIL scale in every panel of this section. Credo scaled from Trijicon's official render using its 5-25 MRAD marks, and checked against Trijicon's dimension sheet (ring 19.3 / 7.5 MRAD).",font=font(22),fill=MUTED); y+=48
y=draw_section(d,img,y,['g2','credo'],["GEN 2 PLxC  -  Raptor G2","TRIJICON CREDO  -  MRAD Seg. Circle","OVERLAY"],
               ["Gen 2","Credo"],(ORANGE,GREEN),31.0,(-12.5,12.5),(-11.5,11.5),fov=(22.1,"Credo 8x field of view (2.53 deg)"))
y=draw_notes(d,y,"Gen 2 PLxC vs. Credo",NOTES_B)
y+=20
d.text((M,y),"Gen 1 MIL values are approximate, measured off the manufacturer render. G2 BDC, wind, lead, horseshoe and ranging-bar values are from the vector art in Primary Arms' G2 manual.",font=font(20),fill=MUTED); y+=28
d.text((M,y),"Credo values are from Trijicon's dimension sheet and spec sheet. Renders: blog.primaryarms.com, trijicon.com.",font=font(20),fill=MUTED)
img=img.crop((0,0,W,y+50))
OUT.mkdir(exist_ok=True)
img.save(OUT / 'plxc_raptor_gen1_vs_g2.png'); print(OUT / 'plxc_raptor_gen1_vs_g2.png', img.size)
