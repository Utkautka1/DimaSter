//! Анимации: плавный курсор с «хвостом», мигание и плавная прокрутка.
//!
//! Движение экспоненциальное и зависит от прошедшего времени, а не от числа
//! кадров, поэтому выглядит одинаково при любой частоте монитора.

use std::time::{Duration, Instant};

use dimaster_config::Effects;

const FRAME: Duration = Duration::from_millis(16);
const BLINK: Duration = Duration::from_millis(530);
// скорость сближения с целью, 1/с
const SPEED: f32 = 28.0;
const TRAIL_STEPS: u16 = 6;

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
    cursor: Option<(f32, f32)>,
    offset: f32,
    last: Instant,
    blink_from: Instant,
    moving: bool,
}

impl Animator {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
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
        let step = if self.moving { elapsed } else { FRAME };
        let factor = 1.0 - (-step.as_secs_f32() * SPEED).exp();
        self.last = now;
        let (cursor, trail) = self.move_cursor(target, effects, factor);
        self.offset *= 1.0 - factor;
        if !effects.smooth_scroll || self.offset.abs() < 0.5 {
            self.offset = 0.0;
        }
        self.moving = cursor != target || self.offset != 0.0;
        let phase = now.duration_since(self.blink_from).as_millis() / BLINK.as_millis();
        let blink_hidden = effects.cursor_blink && focused && phase % 2 == 1;
        Pose {
            cursor,
            trail,
            offset: self.offset.round(),
            blink_hidden,
        }
    }

    fn move_cursor(
        &mut self,
        target: (f32, f32),
        effects: Effects,
        factor: f32,
    ) -> ((f32, f32), Vec<(f32, f32)>) {
        let from = self
            .cursor
            .filter(|_| effects.cursor_animation)
            .unwrap_or(target);
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let mut at = (
            lerp(from.0, target.0, factor),
            lerp(from.1, target.1, factor),
        );
        let distance = (target.0 - at.0).hypot(target.1 - at.1);
        if distance < 0.05 {
            at = target;
        }
        self.cursor = Some(at);
        let mut trail = Vec::new();
        if effects.cursor_trail && distance > 1.5 {
            trail.extend((1..TRAIL_STEPS).map(|i| {
                let t = f32::from(i) / f32::from(TRAIL_STEPS);
                (lerp(from.0, at.0, t), lerp(from.1, at.1, t))
            }));
        }
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
            let passed = now.duration_since(self.blink_from).as_millis() / BLINK.as_millis();
            self.blink_from + BLINK * (passed as u32 + 1)
        })
    }
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
}
