"""Shared GIF/video rendering utilities with Unicode fallback support.

Problem: DejaVu Sans Mono (and most monospace fonts) can't render emoji
(⛔🟢🧪🎮) or circled numbers (①②③), causing □ boxes in GIF output.

Solution: Character-level fallback — replace unrenderable Unicode with
colored ASCII symbols that DejaVu CAN render, while keeping ANSI color.
"""

import os
from PIL import ImageFont, ImageDraw

# ── Fonts ─────────────────────────────────────────────────────

FONT_SIZE = 14
CHAR_W = int(FONT_SIZE * 0.6)
CHAR_H = FONT_SIZE + 4

FONT_PATHS = {
    'mono': '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf',
    'mono_bold': '/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf',
    'cjk': '/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc',
    'sans_bold': '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',
}


def load_fonts():
    """Load primary + fallback fonts."""
    try:
        mono = ImageFont.truetype(FONT_PATHS['mono'], FONT_SIZE)
        mono_bold = ImageFont.truetype(FONT_PATHS['mono_bold'], FONT_SIZE)
        title = ImageFont.truetype(FONT_PATHS['sans_bold'], 18)
    except Exception:
        mono = ImageFont.load_default()
        mono_bold = mono
        title = mono
    try:
        cjk = ImageFont.truetype(FONT_PATHS['cjk'], FONT_SIZE)
    except Exception:
        cjk = mono
    return mono, mono_bold, title, cjk


def char_supported(font, ch):
    """Check if a font can render a character (has non-zero glyph)."""
    try:
        bbox = font.getbbox(ch)
        return bbox[2] > 0
    except Exception:
        return False


# ── Character Replacement Map ─────────────────────────────────
# Maps unsupported Unicode → (replacement_text, ansi_color_code)
# ansi_color_code: None = inherit current fg color

CHAR_REPLACEMENTS = {
    # Status indicators (emoji → colored symbol)
    '⛔': ('\u25cf', 31),   # ● red
    '🟢': ('\u25cf', 32),   # ● green
    # Test markers (circled numbers → bracketed)
    '①': ('[1]', None),
    '②': ('[2]', None),
    '③': ('[3]', None),
    '④': ('[4]', None),
    '⑤': ('[5]', None),
    '⑥': ('[6]', None),
    '⑦': ('[7]', None),
    '⑧': ('[8]', None),
    '⑨': ('[9]', None),
    # Emoji from test scripts → text
    '🧪': ('\u25cf', 95),   # ● bright magenta
    '🎮': ('\u25cf', 94),   # ● bright blue
    '▶': ('\u25b6', None),  # ▶ (should work, but just in case)
    '✅': ('\u2713', 32),   # ✓ green
    '❌': ('\u2717', 31),   # ✗ red
    '⛔': ('\u25cf', 31),   # ● red
}


def is_renderable(ch, font):
    """Check if a character is renderable by the given font."""
    if ord(ch) < 128:
        return True  # ASCII always fine
    return char_supported(font, ch)


def split_char_token(ch, current_fg, font):
    """Return list of (char, color_code) tuples for a character.
    
    If the character is supported by font, return it as-is.
    If not, use the replacement map.
    If not in the map, try CJK font, or fall back to '?'.
    """
    if is_renderable(ch, font):
        return [(ch, None)]
    
    if ch in CHAR_REPLACEMENTS:
        repl_text, repl_color = CHAR_REPLACEMENTS[ch]
        # The replacement might be multi-char like [1]
        result = []
        for rc in repl_text:
            if is_renderable(rc, font):
                result.append((rc, repl_color))
            else:
                result.append(('?', repl_color))
        return result
    
    # Last resort: check CJK font (don't have it here, use '?' fallback)
    return [('?', 90)]  # bright black placeholder


# ── ANSI Parsing ───────────────────────────────────────────────

ANSI_COLORS = {
    30: (0, 0, 0),        31: (205, 49, 49),     32: (13, 188, 121),
    33: (229, 229, 16),   34: (36, 114, 200),     35: (188, 63, 188),
    36: (17, 168, 205),   37: (229, 229, 229),    90: (102, 102, 102),
    91: (241, 76, 76),    92: (35, 209, 139),     93: (245, 245, 67),
    94: (59, 142, 234),   95: (214, 112, 214),    96: (41, 184, 219),
    97: (255, 255, 255),
}
BG_COLORS = {k+10: v for k, v in ANSI_COLORS.items() if k < 40}

# Reverse: RGB → ANSI code for injecting replacement colors
RGB_TO_ANSI = {v: k for k, v in ANSI_COLORS.items()}


def parse_ansi_with_fallback(text, font):
    """Parse ANSI text and apply character fallback replacements.
    
    Returns list of (char, fg_rgb, bg_rgb, bold) tuples,
    where characters have been replaced if unsupported by font.
    """
    result = []
    i = 0
    fg, bg, bold = None, None, False

    while i < len(text):
        if text[i] == '\x1b' and i+1 < len(text) and text[i+1] == '[':
            j = i + 2
            while j < len(text) and text[j] not in 'mGKHJ':
                j += 1
            if j < len(text) and text[j] == 'm':
                codes = text[i+2:j].split(';')
                for c in codes:
                    try:
                        c = int(c)
                    except ValueError:
                        continue
                    if c == 0:
                        fg, bg, bold = None, None, False
                    elif c == 1:
                        bold = True
                    elif c in ANSI_COLORS:
                        fg = ANSI_COLORS[c]
                    elif c in BG_COLORS:
                        bg = BG_COLORS[c]
                i = j + 1
                continue
            i = j + 1
        elif text[i] == '\r':
            i += 1
        elif text[i] == '\n':
            result.append(('\n', fg, bg, bold))
            i += 1
        else:
            ch = text[i]
            tokens = split_char_token(ch, fg, font)
            for tok_ch, override_color_code in tokens:
                tok_fg = fg
                if override_color_code is not None:
                    tok_fg = ANSI_COLORS.get(override_color_code)
                result.append((tok_ch, tok_fg, bg, bold))
            i += 1

    return result
