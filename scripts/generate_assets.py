import os
import struct
import zlib
import math

def create_png(width, height, rgba_data):
    def chunk(tag, data):
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)
    
    ihdr = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # filter type none
        raw.extend(rgba_data[y*width*4 : (y+1)*width*4])
    idat = zlib.compress(bytes(raw), 9)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', ihdr) + chunk(b'IDAT', idat) + chunk(b'IEND', b'')

def render_icon_rgba(size):
    # Create empty transparent canvas
    pixels = bytearray(size * size * 4)

    def set_pixel(x, y, r, g, b, a):
        if 0 <= x < size and 0 <= y < size:
            idx = (y * size + x) * 4
            # Alpha blend over background
            src_a = a / 255.0
            dst_a = pixels[idx + 3] / 255.0
            out_a = src_a + dst_a * (1.0 - src_a)
            if out_a > 0:
                out_r = (r * src_a + pixels[idx] * dst_a * (1.0 - src_a)) / out_a
                out_g = (g * src_a + pixels[idx + 1] * dst_a * (1.0 - src_a)) / out_a
                out_b = (b * src_a + pixels[idx + 2] * dst_a * (1.0 - src_a)) / out_a
                pixels[idx] = int(min(255, max(0, out_r)))
                pixels[idx + 1] = int(min(255, max(0, out_g)))
                pixels[idx + 2] = int(min(255, max(0, out_b)))
                pixels[idx + 3] = int(min(255, max(0, out_a * 255)))

    # Scale-independent coordinates (0.0 to 1.0)
    # Background rounded rect: margins 0.08, corner radius 0.16
    margin = 0.06
    r_corner = 0.16
    bg_r, bg_g, bg_b = 0x24, 0x3B, 0x5A  # Dark steel blue
    border_r, border_g, border_b = 0x41, 0x72, 0xB0 # Accent border

    # Anti-aliased rounded rect rendering
    subsamples = 2 if size <= 32 else 1
    for py in range(size):
        for px in range(size):
            cov_bg = 0
            cov_border = 0
            for sy in range(subsamples):
                for sx in range(subsamples):
                    u = (px + (sx + 0.5) / subsamples) / size
                    v = (py + (sy + 0.5) / subsamples) / size

                    # Box SDF
                    cx = max(margin + r_corner, min(1.0 - margin - r_corner, u))
                    cy = max(margin + r_corner, min(1.0 - margin - r_corner, v))
                    dist = math.hypot(u - cx, v - cy)
                    if dist <= r_corner:
                        cov_bg += 1
                        if dist >= r_corner - 0.05 or u <= margin + 0.05 or u >= 1.0 - margin - 0.05 or v <= margin + 0.05 or v >= 1.0 - margin - 0.05:
                            cov_border += 1

            total_sub = subsamples * subsamples
            if cov_bg > 0:
                alpha = int((cov_bg / total_sub) * 255)
                set_pixel(px, py, bg_r, bg_g, bg_b, alpha)

    # Gutter divider line (u around 0.32)
    gutter_u = 0.32
    gx = int(gutter_u * size)
    for y in range(int(0.18 * size), int(0.82 * size)):
        set_pixel(gx, y, 0x4A, 0x78, 0xB2, 230)

    # Horizontal log lines
    # Line 1: info line
    y1_start = int(0.26 * size)
    y1_h = max(1, int(0.08 * size))
    for y in range(y1_start, y1_start + y1_h):
        for x in range(int(0.16 * size), gx - 1): # line number
            set_pixel(x, y, 0x7E, 0x9B, 0xC3, 200)
        for x in range(gx + 2, int(0.84 * size)): # text line
            set_pixel(x, y, 0xD0, 0xDE, 0xF0, 240)

    # Line 2: highlighted line (search/target line) with accent marker
    y2_start = int(0.44 * size)
    y2_h = max(1, int(0.08 * size))
    for y in range(y2_start, y2_start + y2_h):
        # Amber marker in gutter
        for x in range(int(0.16 * size), gx - 1):
            set_pixel(x, y, 0xF1, 0xCB, 0x69, 255)
        # Search match highlight
        for x in range(gx + 2, int(0.55 * size)):
            set_pixel(x, y, 0xF1, 0xCB, 0x69, 255)
        for x in range(int(0.57 * size), int(0.78 * size)):
            set_pixel(x, y, 0xD0, 0xDE, 0xF0, 220)

    # Line 3: standard line
    y3_start = int(0.62 * size)
    y3_h = max(1, int(0.08 * size))
    for y in range(y3_start, y3_start + y3_h):
        for x in range(int(0.16 * size), gx - 1):
            set_pixel(x, y, 0x7E, 0x9B, 0xC3, 200)
        for x in range(gx + 2, int(0.70 * size)):
            set_pixel(x, y, 0xA8, 0xBF, 0xDC, 220)

    return bytes(pixels)

def generate_all():
    base_dir = r"[local development path removed]"
    branding_dir = os.path.join(base_dir, "assets", "branding")
    icons_dir = os.path.join(base_dir, "assets", "icons")
    resources_dir = os.path.join(base_dir, "assets", "resources")
    os.makedirs(branding_dir, exist_ok=True)
    os.makedirs(icons_dir, exist_ok=True)
    os.makedirs(resources_dir, exist_ok=True)

    # SVG logos
    svg_template = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">
  <rect x="16" y="16" width="224" height="224" rx="36" fill="{bg}" stroke="{border}" stroke-width="8"/>
  <line x1="82" y1="46" x2="82" y2="210" stroke="{gutter}" stroke-width="6" stroke-linecap="round"/>
  <!-- Row 1 -->
  <line x1="40" y1="76" x2="68" y2="76" stroke="{muted}" stroke-width="12" stroke-linecap="round"/>
  <line x1="98" y1="76" x2="214" y2="76" stroke="{text}" stroke-width="12" stroke-linecap="round"/>
  <!-- Row 2: Target row with accent -->
  <line x1="40" y1="128" x2="68" y2="128" stroke="{accent}" stroke-width="12" stroke-linecap="round"/>
  <line x1="98" y1="128" x2="152" y2="128" stroke="{accent}" stroke-width="12" stroke-linecap="round"/>
  <line x1="164" y1="128" x2="204" y2="128" stroke="{text}" stroke-width="12" stroke-linecap="round"/>
  <!-- Row 3 -->
  <line x1="40" y1="180" x2="68" y2="180" stroke="{muted}" stroke-width="12" stroke-linecap="round"/>
  <line x1="98" y1="180" x2="184" y2="180" stroke="{text_subtle}" stroke-width="12" stroke-linecap="round"/>
</svg>"""

    # 1. logo.svg (default branding)
    with open(os.path.join(branding_dir, "logo.svg"), "w", encoding="utf-8") as f:
        f.write(svg_template.format(
            bg="#20242A", border="#315F9A", gutter="#3E4651",
            muted="#657181", text="#E5E8EC", accent="#F1CB69", text_subtle="#A3AFBD"
        ))

    # 2. logo-light.svg
    with open(os.path.join(branding_dir, "logo-light.svg"), "w", encoding="utf-8") as f:
        f.write(svg_template.format(
            bg="#F4F5F7", border="#315F9A", gutter="#DFE3E9",
            muted="#929BA6", text="#232932", accent="#81530A", text_subtle="#505D6D"
        ))

    # 3. logo-dark.svg
    with open(os.path.join(branding_dir, "logo-dark.svg"), "w", encoding="utf-8") as f:
        f.write(svg_template.format(
            bg="#171A1F", border="#91B8ED", gutter="#353C46",
            muted="#737D89", text="#E5E8EC", accent="#F1CB69", text_subtle="#B0BBC8"
        ))

    # Resolutions required: 16, 20, 24, 32, 40, 48, 64, 128, 256
    resolutions = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    png_images = {}
    for res in resolutions:
        rgba = render_icon_rgba(res)
        png_data = create_png(res, res, rgba)
        png_images[res] = png_data
        png_path = os.path.join(icons_dir, f"tylogger-{res}.png")
        with open(png_path, "wb") as f:
            f.write(png_data)
        print(f"Generated {png_path} ({len(png_data)} bytes)")

    # Build genuine multi-resolution ICO file
    ico_path = os.path.join(icons_dir, "tylogger.ico")
    num_images = len(resolutions)
    # Header: idReserved (2), idType (2), idCount (2)
    header = struct.pack('<HHH', 0, 1, num_images)
    entries = []
    offset = 6 + 16 * num_images
    image_bytes = bytearray()

    for res in resolutions:
        png_data = png_images[res]
        w = res if res < 256 else 0
        h = res if res < 256 else 0
        entry = struct.pack('<BBBBHHII', w, h, 0, 0, 1, 32, len(png_data), offset)
        entries.append(entry)
        offset += len(png_data)
        image_bytes.extend(png_data)

    with open(ico_path, "wb") as f:
        f.write(header)
        for e in entries:
            f.write(e)
        f.write(image_bytes)

    print(f"Generated {ico_path} ({os.path.getsize(ico_path)} bytes)")

    # 4. Generate Windows Resource script .rc
    rc_content = """#pragma code_page(65001)
1 ICON "..\\\\icons\\\\tylogger.ico"

1 VERSIONINFO
FILEVERSION 0,2,0,0
PRODUCTVERSION 0,2,0,0
FILEFLAGSMASK 0x3fL
#ifdef _DEBUG
FILEFLAGS 0x1L
#else
FILEFLAGS 0x0L
#endif
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "Tylogger Contributors\\0"
            VALUE "FileDescription", "Tylogger Log Analysis Tool\\0"
            VALUE "FileVersion", "0.2.0.0\\0"
            VALUE "InternalName", "tylogger\\0"
            VALUE "LegalCopyright", "Copyright (c) 2026 Tylogger Contributors. MIT License.\\0"
            VALUE "OriginalFilename", "tylogger.exe\\0"
            VALUE "ProductName", "Tylogger\\0"
            VALUE "ProductVersion", "0.2.0\\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"""
    rc_path = os.path.join(resources_dir, "tylogger.rc")
    with open(rc_path, "w", encoding="utf-8") as f:
        f.write(rc_content)
    print(f"Generated {rc_path}")

if __name__ == '__main__':
    generate_all()
