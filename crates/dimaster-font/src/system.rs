//! Загрузка шрифтов и кэш растров.

use std::collections::HashMap;
use std::fmt;

use fontdb::{Database, ID};
use ttf_parser::Face;

use crate::discover;
use crate::raster::{Bitmap, Outline};

/// Размеры ячейки в физических пикселях.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub cell_width: u32,
    pub cell_height: u32,
    /// Базовая линия от верха ячейки.
    pub baseline: i32,
    pub line_thickness: u32,
}

/// Что растеризовать.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub ch: char,
    pub bold: bool,
    pub italic: bool,
}

/// Не нашлось ни одного пригодного шрифта.
#[derive(Debug)]
pub struct FontError;

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("не найден пригодный моноширинный шрифт (задайте DIMASTER_FONT)")
    }
}

impl std::error::Error for FontError {}

/// Системные шрифты, выбранное семейство и кэш глифов.
pub struct FontSystem {
    db: Database,
    family: String,
    // кегль в физических пикселях
    size: f32,
    metrics: Metrics,
    // None — для символа нет шрифта; кэшируем, чтобы не сканировать повторно
    fallbacks: HashMap<char, Option<ID>>,
    glyphs: HashMap<GlyphKey, Bitmap>,
}

impl FontSystem {
    pub fn new(size: f32) -> Result<Self, FontError> {
        let mut db = Database::new();
        db.load_system_fonts();
        let family = discover::default_family(&db).ok_or(FontError)?;
        let id = discover::find_face(&db, &family, false, false).ok_or(FontError)?;
        let metrics = db
            .with_face_data(id, |data, index| {
                Some(measure(&Face::parse(data, index).ok()?, size))
            })
            .flatten()
            .ok_or(FontError)?;
        log::info!("Шрифт: {family}, {size} px, ячейка {metrics:?}");
        let (fallbacks, glyphs) = Default::default();
        Ok(Self {
            db,
            family,
            size,
            metrics,
            fallbacks,
            glyphs,
        })
    }

    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    /// Меняет кегль (масштаб монитора) без повторного сканирования шрифтов.
    pub fn set_size(&mut self, size: f32) {
        let Some(id) = discover::find_face(&self.db, &self.family, false, false) else {
            return;
        };
        let metrics = self.db.with_face_data(id, |data, index| {
            Some(measure(&Face::parse(data, index).ok()?, size))
        });
        if let Some(Some(metrics)) = metrics {
            self.size = size;
            self.metrics = metrics;
            self.glyphs.clear();
        }
    }

    /// Растр символа; пустой, если ни в одном шрифте нет контура.
    pub fn glyph(&mut self, key: GlyphKey) -> &Bitmap {
        if !self.glyphs.contains_key(&key) {
            let bitmap = self.render(key).unwrap_or_default();
            self.glyphs.insert(key, bitmap);
        }
        &self.glyphs[&key]
    }

    fn render(&mut self, key: GlyphKey) -> Option<Bitmap> {
        let primary = discover::find_face(&self.db, &self.family, key.bold, key.italic)?;
        if let Some(bitmap) = self.render_with(primary, key.ch) {
            return Some(bitmap);
        }
        let db = &self.db;
        let id = *self
            .fallbacks
            .entry(key.ch)
            .or_insert_with(|| discover::fallback_face(db, key.ch));
        self.render_with(id?, key.ch)
    }

    fn render_with(&self, id: ID, ch: char) -> Option<Bitmap> {
        self.db.with_face_data(id, |data, index| {
            let face = Face::parse(data, index).ok()?;
            let glyph = face.glyph_index(ch)?;
            let mut outline = Outline::new(self.size / f32::from(face.units_per_em()));
            face.outline_glyph(glyph, &mut outline);
            Some(outline.rasterize())
        })?
    }
}

fn measure(face: &Face<'_>, size: f32) -> Metrics {
    let scale = size / f32::from(face.units_per_em());
    let advance = face
        .glyph_index('M')
        .and_then(|g| face.glyph_hor_advance(g))
        .unwrap_or(face.units_per_em() / 2);
    let ascender = f32::from(face.ascender()) * scale;
    let descender = f32::from(face.descender()) * scale;
    let gap = f32::from(face.line_gap()) * scale;
    let thickness = face
        .underline_metrics()
        .map_or(1.0, |m| f32::from(m.thickness) * scale);
    Metrics {
        cell_width: (f32::from(advance) * scale).round().max(1.0) as u32,
        cell_height: (ascender - descender + gap).ceil().max(1.0) as u32,
        baseline: (ascender + gap / 2.0).round() as i32,
        line_thickness: thickness.round().max(1.0) as u32,
    }
}
