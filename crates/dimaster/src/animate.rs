//! Анимации: плавный курсор с «хвостом», мигание и плавная прокрутка.
//!
//! Движение экспоненциальное и зависит от прошедшего времени, а не от числа
//! кадров, поэтому выглядит одинаково при любой частоте монитора.

use std::time::{Duration, Instant};

use dimaster_config::{CursorLook, Effects};

const FRAME: Duration = Duration::from_millis(16);

/// Что нарисовать в этом кадре.
#[derive(Debug, Default)]
pub(crate) struct Pose {
    /// (строка, колонка) курсора.
    pub(crate) cursor: (f32, f32),
    pub(crate) trail: Vec<(f32, f32)>,
    // сдвиг содержимого в пикселях
    pub(crate) offset: f32,
    pub(crate) blink_hidden: bool,
}

#[derive(Debug)]
pub(crate) struct Animator {
    pub(crate) look: CursorLook,
    cursor: Option<(f32, f32)>,
    offset: f32,
    last: Instant,
    blink_from: Instant,
    moving: bool,
}

impl Animator {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            look: CursorLook::default(),
            cursor: None,
            offset: 0.0,
            last: now,
            blink_from: now,
            moving: false,
        }
    }

    /// После ввода курсор снова виден.
    pub(crate) fn restart_blink(&mut self, now: Instant) {
        self.blink_from = now;
    }

    /// Начинает плавную прокрутку: содержимое стартует со сдвигом `pixels`.
    pub(crate) fn scroll(&mut self, pixels: f32, limit: f32) {
        self.offset = (self.offset + pixels).clamp(-limit, limit);
    }

    pub(crate) fn tick(
        &mut self,
        target: (f32, f32),
        effects: Effects,
        focused: bool,
        now: Instant,
    ) -> Pose {
        // после простоя не делаем один огромный шаг
        let elapsed = now.duration_since(self.last).min(Duration::from_millis(50));
        let step = if self.moving { elapsed } else { FRAME }.as_secs_f32();
        let factor = 1.0 - (-step * self.look.speed).exp();
        self.last = now;
        let (cursor, trail) = self.move_cursor(target, effects, (factor, step));
        self.offset *= 1.0 - factor;
        if !effects.smooth_scroll || self.offset.abs() < 0.5 {
            self.offset = 0.0;
        }
        self.moving = cursor != target || self.offset != 0.0;
        let phase = now.duration_since(self.blink_from).as_millis() / self.blink().as_millis();
        let blink_hidden = effects.cursor_blink && focused && phase % 2 == 1;
        Pose {
            cursor,
            trail,
            offset: self.offset.round(),
            blink_hidden,
        }
    }

    fn blink(&self) -> Duration {
        Duration::from_millis(u64::from(self.look.blink_interval.max(1)))
    }

    // `step` — доля пути для плавного движения и секунды для равномерного
    fn advance(&self, from: (f32, f32), to: (f32, f32), step: (f32, f32)) -> (f32, f32) {
        let distance = (to.0 - from.0).hypot(to.1 - from.1);
        let part = match self.look.linear {
            true => (self.look.speed * step.1 / distance.max(f32::EPSILON)).min(1.0),
            false => step.0,
        };
        let at = (lerp(from.0, to.0, part), lerp(from.1, to.1, part));
        match (to.0 - at.0).hypot(to.1 - at.1) < 0.05 {
            true => to,
            false => at,
        }
    }

    fn trail(&self, from: (f32, f32), at: (f32, f32)) -> Vec<(f32, f32)> {
        let copies = self.look.trail_length;
        let part = |i: u16| f32::from(i) / f32::from(copies + 1);
        let points =
            (1..=copies).map(|i| (lerp(from.0, at.0, part(i)), lerp(from.1, at.1, part(i))));
        points.collect()
    }

    fn move_cursor(
        &mut self,
        target: (f32, f32),
        effects: Effects,
        step: (f32, f32),
    ) -> ((f32, f32), Vec<(f32, f32)>) {
        let from = self
            .cursor
            .filter(|_| effects.cursor_animation)
            .unwrap_or(target);
        let at = self.advance(from, target, step);
        self.cursor = Some(at);
        let far = (target.0 - at.0).hypot(target.1 - at.1) > self.look.trail_distance;
        let trail = match effects.cursor_trail && far {
            true => self.trail(from, at),
            false => Vec::new(),
        };
        (at, trail)
    }

    /// Когда нужен следующий кадр; `None` — всё неподвижно.
    pub(crate) fn next_frame(
        &self,
        effects: Effects,
        focused: bool,
        now: Instant,
    ) -> Option<Instant> {
        if self.moving {
            return Some(now + FRAME);
        }
        (effects.cursor_blink && focused).then(|| {
            let blink = self.blink();
            let passed = now.duration_since(self.blink_from).as_millis() / blink.as_millis();
            self.blink_from + blink * (passed as u32 + 1)
        })
    }
}

fn lerp(from: f32, to: f32, part: f32) -> f32 {
    from + (to - from) * part
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_converges_and_stops() {
        let start = Instant::now();
        let mut animator = Animator::new(start);
        let effects = Effects {
            cursor_trail: true,
            ..Effects::default()
        };
        animator.tick((0.0, 0.0), effects, true, start);
        let first = animator.tick((0.0, 20.0), effects, true, start + FRAME);
        assert!(first.cursor.1 > 0.0 && first.cursor.1 < 20.0);
        assert!(!first.trail.is_empty());
        let end = (2..60)
            .map(|i| animator.tick((0.0, 20.0), effects, true, start + FRAME * i))
            .last();
        assert_eq!(end.unwrap().cursor, (0.0, 20.0));
        assert_eq!(animator.next_frame(effects, true, start + FRAME * 60), None);
    }

    #[test]
    fn linear_motion_keeps_its_speed() {
        let start = Instant::now();
        let mut animator = Animator::new(start);
        (animator.look.linear, animator.look.speed) = (true, 100.0);
        let effects = Effects::default();
        animator.tick((0.0, 0.0), effects, true, start);
        let first = animator.tick((0.0, 20.0), effects, true, start + FRAME);
        assert!((first.cursor.1 - 1.6).abs() < 0.01, "{:?}", first.cursor);
    }
}
