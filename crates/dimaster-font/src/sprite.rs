//! Свои рамки, блоки, шрифт Брайля и «стрелки» Powerline.
//!
//! Глифы из шрифта не всегда доходят до краёв ячейки, и на рамках появляются
//! щели. Эти символы строятся геометрически точно по размеру ячейки, поэтому
//! стыкуются при любом шрифте и межстрочном интервале.

use crate::{Bitmap, Metrics};

/// Какие символы рисуются без шрифта.
pub(crate) fn is_sprite(ch: char) -> bool {
    matches!(ch, '\u{2500}'..='\u{259f}' | '\u{2800}'..='\u{28ff}')
        || matches!(ch, '\u{e0b0}' | '\u{e0b2}' | '\u{e0b4}' | '\u{e0b6}')
}

// толщина концов линии из центра: влево, вправо, вверх, вниз
// (0 — нет, 1 — тонкая, 2 — жирная, 3 — двойная); начиная с U+250C
const ARMS: &[u8; 192] = b"\
    0101020101020202100120011002200201100210012002201010201010202020\
    0111021101210112012202210212022210112011102110121022202120122022\
    1101210112012201110221021202220211102110121022101120212012202220";
const MIXED: &[u8; 64] = b"\
    1111211112112211112111121122212112212112121222212212212212222222";
// двойные: с U+2550 по U+256C
const DOUBLE: &[u8; 116] = b"\
    3300003303010103030330011003300303100130033030101030303003110133\
    0333301110333033330111033303331011303330331111333333";

struct Canvas {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl Canvas {
    fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, alpha: u8) {
        for row in y.min(self.height)..(y + height).min(self.height) {
            let start = row * self.width + x.min(self.width);
            let end = row * self.width + (x + width).min(self.width);
            self.data[start..end].fill(alpha);
        }
    }

    // прямоугольник по краям: [x0, x1) × [y0, y1)
    fn span(&mut self, x0: usize, x1: usize, y0: usize, y1: usize) {
        self.fill(x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0), 255);
    }

    // сглаживание по 4×4 точкам внутри каждого пикселя
    fn shape(&mut self, inside: impl Fn(f32, f32) -> bool) {
        for y in 0..self.height {
            for x in 0..self.width {
                let hits = (0..16)
                    .filter(|i| {
                        let sub = |v: usize, k: usize| v as f32 + (k as f32 + 0.5) / 4.0;
                        inside(sub(x, i % 4), sub(y, i / 4))
                    })
                    .count();
                let alpha = (hits * 255 / 16) as u8;
                let pixel = &mut self.data[y * self.width + x];
                *pixel = (*pixel).max(alpha);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Arms {
    left: u8,
    right: u8,
    up: u8,
    down: u8,
}

// строка таблицы: четыре цифры на символ
fn digits(table: &[u8], index: u32) -> Option<Arms> {
    let at = index as usize * 4;
    let four = table.get(at..at + 4)?;
    let value = |i: usize| four[i] - b'0';
    Some(Arms {
        left: value(0),
        right: value(1),
        up: value(2),
        down: value(3),
    })
}

// простые линии и «обрубки» с U+2574
const STRAIGHT: &[u8; 16] = b"1100220000110022";
const ENDS: &[u8; 48] = b"100000100100000120000020020000021200001221000021";

fn arms_of(code: u32) -> Option<Arms> {
    match code {
        0x2500..=0x2503 => digits(STRAIGHT, code - 0x2500),
        0x250c..=0x253b => digits(ARMS, code - 0x250c),
        0x253c..=0x254b => digits(MIXED, code - 0x253c),
        0x2550..=0x256c => digits(DOUBLE, code - 0x2550),
        0x2574..=0x257f => digits(ENDS, code - 0x2574),
        _ => None,
    }
}

struct Pen {
    // толщина тонкой линии и центр ячейки
    light: usize,
    cx: usize,
    cy: usize,
}

impl Pen {
    fn thickness(&self, weight: u8) -> usize {
        if weight == 2 {
            self.light * 2 + 1
        } else {
            self.light
        }
    }

    // начало полосы шириной `size` с центром в `center`
    fn start(center: usize, size: usize) -> usize {
        center.saturating_sub(size / 2)
    }

    fn single(&self, canvas: &mut Canvas, arms: Arms) {
        let (w, h) = (canvas.width, canvas.height);
        let vertical = self.thickness(arms.up.max(arms.down).max(1));
        let horizontal = self.thickness(arms.left.max(arms.right).max(1));
        // концы заходят за центр на толщину поперечной линии, чтобы не было выемок
        let (vx, hy) = (
            Self::start(self.cx, vertical),
            Self::start(self.cy, horizontal),
        );
        for (weight, x0, x1) in [(arms.left, 0, vx + vertical), (arms.right, vx, w)] {
            if weight == 1 || weight == 2 {
                let t = self.thickness(weight);
                canvas.span(x0, x1, Self::start(self.cy, t), Self::start(self.cy, t) + t);
            }
        }
        for (weight, y0, y1) in [(arms.up, 0, hy + horizontal), (arms.down, hy, h)] {
            if weight == 1 || weight == 2 {
                let t = self.thickness(weight);
                canvas.span(Self::start(self.cx, t), Self::start(self.cx, t) + t, y0, y1);
            }
        }
    }

    // где обрывается полоса двойной линии: у внутреннего угла — раньше
    fn stop(center: usize, gap: usize, same: bool, other: bool, forward: bool) -> usize {
        let near = if forward { center - gap } else { center + gap };
        let far = if forward { center + gap } else { center - gap };
        match (same, other) {
            (true, _) => near,
            (false, true) => far,
            _ => center,
        }
    }

    fn double_horizontal(&self, canvas: &mut Canvas, arms: Arms) {
        let (gap, t) = (self.light + 1, self.light);
        let (up, down) = (arms.up == 3, arms.down == 3);
        for (offset_up, y) in [(true, self.cy - gap), (false, self.cy + gap)] {
            let (same, other) = if offset_up { (up, down) } else { (down, up) };
            let y = Self::start(y, t);
            if arms.left == 3 {
                let end = Self::stop(self.cx, gap, same, other, true);
                canvas.span(0, end + t - t / 2, y, y + t);
            }
            if arms.right == 3 {
                let begin = Self::stop(self.cx, gap, same, other, false);
                canvas.span(Self::start(begin, t), canvas.width, y, y + t);
            }
        }
    }

    fn double_vertical(&self, canvas: &mut Canvas, arms: Arms) {
        let (gap, t) = (self.light + 1, self.light);
        let (left, right) = (arms.left == 3, arms.right == 3);
        for (offset_left, x) in [(true, self.cx - gap), (false, self.cx + gap)] {
            let (same, other) = if offset_left {
                (left, right)
            } else {
                (right, left)
            };
            let x = Self::start(x, t);
            if arms.up == 3 {
                let end = Self::stop(self.cy, gap, same, other, true);
                canvas.span(x, x + t, 0, end + t - t / 2);
            }
            if arms.down == 3 {
                let begin = Self::stop(self.cy, gap, same, other, false);
                canvas.span(x, x + t, Self::start(begin, t), canvas.height);
            }
        }
    }

    fn lines(&self, canvas: &mut Canvas, arms: Arms) {
        self.single(canvas, arms);
        self.double_horizontal(canvas, arms);
        self.double_vertical(canvas, arms);
    }

    // пунктир: столько штрихов на ячейку
    fn dashes(&self, canvas: &mut Canvas, code: u32) {
        let (index, count) = match code {
            0x2504..=0x2507 => (code - 0x2504, 3),
            0x2508..=0x250b => (code - 0x2508, 4),
            _ => (code - 0x254c, 2),
        };
        let t = self.thickness(1 + (index % 2) as u8);
        let vertical = index >= 2;
        let length = if vertical {
            canvas.height
        } else {
            canvas.width
        };
        for i in 0..count {
            let from = i * length / count;
            let to = from + (length / count * 2 / 3).max(1);
            if vertical {
                let x = Self::start(self.cx, t);
                canvas.span(x, x + t, from, to);
            } else {
                let y = Self::start(self.cy, t);
                canvas.span(from, to, y, y + t);
            }
        }
    }

    // скруглённый угол: четверть окружности и прямой хвост до края
    fn arc(&self, canvas: &mut Canvas, code: u32) {
        let right = matches!(code, 0x256d | 0x2570);
        let down = matches!(code, 0x256d | 0x256e);
        let half = self.light as f32 / 2.0;
        let line = |c: usize| (Self::start(c, self.light)) as f32 + half;
        let (cx, cy) = (line(self.cx), line(self.cy));
        let radius = (canvas.width as f32 / 2.0).min(canvas.height as f32 / 2.0);
        let ox = if right { cx + radius } else { cx - radius };
        let oy = if down { cy + radius } else { cy - radius };
        canvas.shape(|x, y| {
            let beyond_x = if right { x >= ox } else { x <= ox };
            let beyond_y = if down { y >= oy } else { y <= oy };
            let ring = ((x - ox).hypot(y - oy) - radius).abs() <= half;
            let corner = !beyond_x && !beyond_y && ring;
            let tail_x = beyond_x && (y - cy).abs() <= half;
            let tail_y = beyond_y && (x - cx).abs() <= half;
            corner || tail_x || tail_y
        });
    }

    fn diagonal(&self, canvas: &mut Canvas, code: u32) {
        let (w, h) = (canvas.width as f32, canvas.height as f32);
        let half = self.light as f32 / 2.0 + 0.2;
        let length = w.hypot(h);
        canvas.shape(|x, y| {
            let rising = (h * x + w * y - w * h).abs() / length <= half;
            let falling = (h * x - w * y).abs() / length <= half;
            (code != 0x2572 && rising) || (code != 0x2571 && falling)
        });
    }
}

// квадранты U+2596..U+259F: биты — верх-лево, верх-право, низ-лево, низ-право
const QUADRANTS: [u8; 10] = [
    0b0100, 0b1000, 0b0001, 0b1101, 0b1001, 0b0111, 0b1011, 0b0010, 0b0110, 0b1110,
];

fn block(canvas: &mut Canvas, code: u32) {
    let (w, h) = (canvas.width, canvas.height);
    let eighth = |size: usize, parts: u32| (size * parts as usize + 4) / 8;
    match code {
        0x2580 => canvas.span(0, w, 0, h / 2),
        0x2581..=0x2588 => canvas.span(0, w, h - eighth(h, code - 0x2580), h),
        0x2589..=0x258f => canvas.span(0, eighth(w, 0x2590 - code), 0, h),
        0x2590 => canvas.span(w / 2, w, 0, h),
        0x2591..=0x2593 => canvas.fill(0, 0, w, h, (64 * (code - 0x2590)) as u8),
        0x2594 => canvas.span(0, w, 0, eighth(h, 1)),
        0x2595 => canvas.span(w - eighth(w, 1), w, 0, h),
        _ => {
            let mask = QUADRANTS[(code - 0x2596) as usize];
            for (bit, x, y) in [(1, 0, 0), (2, 1, 0), (4, 0, 1), (8, 1, 1)] {
                if mask & bit != 0 {
                    let (x0, y0) = (x * w / 2, y * h / 2);
                    canvas.span(x0, (x + 1) * w / 2, y0, (y + 1) * h / 2);
                }
            }
        }
    }
}

// точки 1–3 и 7 в левом столбце, 4–6 и 8 в правом
fn braille(canvas: &mut Canvas, code: u32) {
    const PLACES: [(usize, usize); 8] = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (1, 1),
        (1, 2),
        (0, 3),
        (1, 3),
    ];
    let (w, h) = (canvas.width, canvas.height);
    let size = (w / 4).min(h / 8).max(1);
    for (bit, (column, row)) in PLACES.into_iter().enumerate() {
        if (code - 0x2800) & (1 << bit) != 0 {
            let x = (2 * column + 1) * w / 4;
            let y = (2 * row + 1) * h / 8;
            canvas.fill(
                x.saturating_sub(size / 2),
                y.saturating_sub(size / 2),
                size,
                size,
                255,
            );
        }
    }
}

// треугольники и полукруги Powerline занимают ячейку целиком
fn powerline(canvas: &mut Canvas, code: u32) {
    let (w, h) = (canvas.width as f32, canvas.height as f32);
    canvas.shape(|x, y| {
        let from_center = (y - h / 2.0).abs() / (h / 2.0);
        match code {
            0xe0b0 => x <= w * (1.0 - from_center),
            0xe0b2 => x >= w * from_center,
            0xe0b4 => (x / w).hypot(from_center) <= 1.0,
            _ => ((w - x) / w).hypot(from_center) <= 1.0,
        }
    });
}

fn draw_into(canvas: &mut Canvas, code: u32, pen: &Pen) {
    match code {
        0x2504..=0x250b | 0x254c..=0x254f => pen.dashes(canvas, code),
        0x256d..=0x2570 => pen.arc(canvas, code),
        0x2571..=0x2573 => pen.diagonal(canvas, code),
        0x2580..=0x259f => block(canvas, code),
        0x2800..=0x28ff => braille(canvas, code),
        0xe0b0..=0xe0b6 => powerline(canvas, code),
        _ => {
            if let Some(arms) = arms_of(code) {
                pen.lines(canvas, arms);
            }
        }
    }
}

/// Растр размером ровно в ячейку.
pub(crate) fn draw(ch: char, metrics: Metrics) -> Bitmap {
    let (width, height) = (metrics.cell_width as usize, metrics.cell_height as usize);
    let mut canvas = Canvas {
        width,
        height,
        data: vec![0; width * height],
    };
    let light = metrics.line_thickness.max(1) as usize;
    let pen = Pen {
        light,
        cx: width / 2,
        cy: height / 2,
    };
    draw_into(&mut canvas, u32::from(ch), &pen);
    Bitmap {
        left: 0,
        top: metrics.baseline,
        width,
        height,
        data: canvas.data,
        color: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRICS: Metrics = Metrics {
        cell_width: 10,
        cell_height: 20,
        baseline: 15,
        line_thickness: 1,
    };

    fn lit(ch: char, x: usize, y: usize) -> bool {
        draw(ch, METRICS).data[y * 10 + x] > 128
    }

    // концы доходят до краёв — соседние ячейки стыкуются без щелей
    #[test]
    fn lines_reach_cell_edges() {
        assert!(lit('─', 0, 10) && lit('─', 9, 10) && !lit('─', 5, 0));
        assert!(lit('│', 5, 0) && lit('│', 5, 19));
        assert!(lit('┌', 9, 10) && lit('┌', 5, 19) && !lit('┌', 0, 10) && !lit('┌', 5, 0));
        assert!(lit('╭', 9, 10) && lit('╭', 5, 19) && !lit('╭', 5, 10));
    }

    #[test]
    fn every_box_character_draws_something() {
        for code in (0x2500..=0x259f).chain(0x2801..=0x28ff) {
            let ch = char::from_u32(code).unwrap();
            assert!(
                draw(ch, METRICS).data.iter().any(|&a| a > 0),
                "пусто: {ch} U+{code:04X}"
            );
        }
    }

    #[test]
    fn double_corner_and_blocks() {
        // у ╔ две вложенные полосы: внешний угол выше и левее внутреннего
        assert!(lit('╔', 3, 8) && lit('╔', 9, 8) && lit('╔', 7, 12) && !lit('╔', 5, 10));
        assert!(lit('▄', 0, 19) && !lit('▄', 0, 0));
        assert!(lit('▌', 0, 0) && !lit('▌', 9, 0));
    }
}
