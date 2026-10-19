//! Renders the tray icon: a battery outline filled to the charge level with
//! the percentage written on top. Output is straight RGBA, row major.

// 3x5 bitmap digits, one row per u8 (3 low bits used, MSB = left column).
const GLYPHS: [[u8; 5]; 11] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
    [0b111, 0b001, 0b011, 0b000, 0b010], // ?
];
const QUESTION: usize = 10;

struct Canvas {
    size: usize,
    px: Vec<[u8; 4]>,
}

impl Canvas {
    fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, c: [u8; 4]) {
        for yy in y..(y + h).min(self.size) {
            for xx in x..(x + w).min(self.size) {
                self.px[yy * self.size + xx] = c;
            }
        }
    }
}

/// `percent = None` draws a grey battery with a question mark.
pub fn render(size: usize, percent: Option<u8>) -> Vec<u8> {
    let s = size as f32 / 32.0;
    let u = |v: f32| (v * s).round() as usize;
    let mut c = Canvas { size, px: vec![[0; 4]; size * size] };

    let outline = [230, 230, 230, 255];
    let fill = match percent {
        None => [110, 110, 110, 255],
        Some(p) if p <= 15 => [220, 50, 50, 255],
        Some(p) if p <= 35 => [235, 150, 30, 255],
        Some(_) => [60, 180, 75, 255],
    };

    // Body 0..28 x 6..26, terminal 28..31 x 12..20 (in 32px units).
    let (bx, by, bw, bh, t) = (0, u(6.0), u(28.0), u(20.0), u(2.0).max(1));
    c.rect(bx, by, bw, t, outline);
    c.rect(bx, by + bh - t, bw, t, outline);
    c.rect(bx, by, t, bh, outline);
    c.rect(bx + bw - t, by, t, bh, outline);
    c.rect(bx + bw, u(12.0), u(3.0), u(8.0), outline);

    let inner_w = bw - 2 * t;
    let level = percent.map_or(inner_w, |p| (inner_w * p.min(100) as usize).div_ceil(100));
    c.rect(bx + t, by + t, level, bh - 2 * t, fill);

    // Text: up to 3 digits, scaled to fit inside the body, with a dark outline.
    let glyphs: Vec<usize> = match percent {
        Some(p) => p.min(100).to_string().bytes().map(|b| (b - b'0') as usize).collect(),
        None => vec![QUESTION],
    };
    let px = (if glyphs.len() == 3 { 2.0 } else { 3.0 } * s).max(1.0) as usize;
    let text_w = glyphs.len() * 3 * px + (glyphs.len() - 1) * px;
    let x0 = bx + (bw - text_w) / 2;
    let y0 = by + (bh - 5 * px) / 2;
    for pass in 0..2 {
        for (i, g) in glyphs.iter().enumerate() {
            for (row, bits) in GLYPHS[*g].iter().enumerate() {
                for col in 0..3 {
                    if bits & (0b100 >> col) == 0 {
                        continue;
                    }
                    let x = x0 + i * 4 * px + col * px;
                    let y = y0 + row * px;
                    if pass == 0 {
                        c.rect(x.saturating_sub(1), y.saturating_sub(1), px + 2, px + 2, [0, 0, 0, 230]);
                    } else {
                        c.rect(x, y, px, px, [255, 255, 255, 255]);
                    }
                }
            }
        }
    }

    c.px.into_iter().flatten().collect()
}
