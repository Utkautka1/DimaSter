//! Загрузка шрифтов и кэш растров.

use dimaster_i18n::tr;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

use fontdb::{Database, ID};
use rustybuzz::ttf_parser::{self, Face};

use crate::raster::{Bitmap, Outline};
use crate::{GlyphId, ShapedGlyph, color, discover, sprite};

/// Размеры ячейки в физических пикселях.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub cell_width: u32,
    pub cell_height: u32,
    /// Базовая линия от верха ячейки.
    pub baseline: i32,
    pub line_thickness: u32,
}

/// Не нашлось ни одного пригодного шрифта.
#[derive(Debug)]
pub struct FontError;

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&tr(
            "не найден пригодный моноширинный шрифт (задайте DIMASTER_FONT)",
        ))
    }
}

impl std::error::Error for FontError {}

/// Системные шрифты, выбранное семейство и кэш глифов.
pub struct FontSystem {
    pub(crate) db: Database,
    pub(crate) family: String,
    // свои семейства для жирного [0], курсива [1] и жирного курсива [2]
    styles: [Option<String>; 3],
    // кегль в физических пикселях
    pub(crate) size: f32,
    // множитель высоты строки
    line_height: f32,
    pub(crate) ligatures: bool,
    metrics: Metrics,
    // None — для символа нет шрифта; кэшируем, чтобы не сканировать повторно
    pub(crate) fallbacks: HashMap<(char, bool), Option<ID>>,
    // результат шейпинга по тексту, жирности и курсиву
    pub(crate) shaped: HashMap<(String, bool, bool), Rc<[ShapedGlyph]>>,
    // растры по глифу и числу занимаемых ячеек
    glyphs: HashMap<(GlyphId, u32), Bitmap>,
}

impl FontSystem {
    pub fn new(size: f32) -> Result<Self, FontError> {
        Self::with_settings(None, size, &[])
    }

    /// `family = None` — выбор fontconfig; `dirs` — свои каталоги шрифтов.
    pub fn with_settings(
        family: Option<&str>,
        size: f32,
        dirs: &[PathBuf],
    ) -> Result<Self, FontError> {
        let mut db = Database::new();
        db.load_system_fonts();
        dirs.iter().for_each(|dir| db.load_fonts_dir(dir));
        let family = choose_family(&db, family).ok_or(FontError)?;
        let id = discover::find_face(&db, &family, false, false).ok_or(FontError)?;
        let metrics = db
            .with_face_data(id, |data, index| {
                Some(measure(&Face::parse(data, index).ok()?, size))
            })
            .flatten()
            .ok_or(FontError)?;
        log::info!("Шрифт: {family}, {size} px, ячейка {metrics:?}");
        Ok(Self::assemble(db, family, size, metrics))
    }

    fn assemble(db: Database, family: String, size: f32, metrics: Metrics) -> Self {
        let (fallbacks, shaped, glyphs) = Default::default();
        Self {
            db,
            family,
            styles: Default::default(),
            size,
            line_height: 1.0,
            ligatures: true,
            metrics,
            fallbacks,
            shaped,
            glyphs,
        }
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
            self.metrics = spaced(metrics, self.line_height);
            self.glyphs.clear();
            self.shaped.clear();
        }
    }

    /// Межстрочный интервал и лигатуры из настроек.
    pub fn set_style(&mut self, line_height: f32, ligatures: bool) {
        self.line_height = line_height;
        self.ligatures = ligatures;
        self.set_size(self.size);
    }

    /// Отдельные семейства: жирный, курсив, жирный курсив. `None` — то же
    /// семейство, что и у обычного текста.
    pub fn set_style_families(&mut self, families: [Option<String>; 3]) {
        self.styles = families;
        self.glyphs.clear();
        self.shaped.clear();
    }

    pub(crate) fn face_of_style(&self, bold: bool, italic: bool) -> Option<ID> {
        let index = match (bold, italic) {
            (true, false) => Some(0),
            (false, true) => Some(1),
            (true, true) => Some(2),
            (false, false) => None,
        };
        let custom = index.and_then(|i| self.styles[i].as_deref());
        discover::find_face(&self.db, custom.unwrap_or(&self.family), bold, italic)
    }

    /// Растр глифа; `cells` нужен цветным эмодзи, чтобы вписаться в ячейки.
    pub fn glyph(&mut self, id: GlyphId, cells: u32) -> &Bitmap {
        if !self.glyphs.contains_key(&(id, cells)) {
            let bitmap = self.render(id, cells).unwrap_or_default();
            self.glyphs.insert((id, cells), bitmap);
        }
        &self.glyphs[&(id, cells)]
    }

    fn render(&self, id: GlyphId, cells: u32) -> Option<Bitmap> {
        if id.face == ID::dummy() {
            let ch = char::from_u32(u32::from(id.glyph))?;
            return Some(sprite::draw(ch, self.metrics));
        }
        self.db.with_face_data(id.face, |data, index| {
            let face = Face::parse(data, index).ok()?;
            let glyph = ttf_parser::GlyphId(id.glyph);
            if let Some(bitmap) = color::render(&face, glyph, cells, self.metrics) {
                return Some(bitmap);
            }
            let scale = self.size / f32::from(face.units_per_em());
            let bitmap = outline(&face, glyph, scale);
            let room = (cells * self.metrics.cell_width) as usize;
            if !id.fit || bitmap.width <= room + 1 {
                return Some(bitmap);
            }
            let smaller = outline(&face, glyph, scale * room as f32 / bitmap.width as f32);
            Some(centered(smaller, room))
        })?
    }
}

fn outline(face: &Face<'_>, glyph: ttf_parser::GlyphId, scale: f32) -> Bitmap {
    let mut outline = Outline::new(scale);
    face.outline_glyph(glyph, &mut outline);
    outline.rasterize()
}

// уменьшенный значок ставится по центру отведённой ширины
fn centered(mut bitmap: Bitmap, room: usize) -> Bitmap {
    bitmap.left = (room.saturating_sub(bitmap.width) / 2) as i32;
    bitmap
}

// опечатка в имени шрифта не должна оставлять терминал без текста
fn choose_family(db: &Database, wanted: Option<&str>) -> Option<String> {
    if let Some(name) = wanted {
        let installed = db
            .faces()
            .any(|f| f.families.iter().any(|(n, _)| n == name));
        if installed {
            return Some(name.to_owned());
        }
        log::warn!("Шрифт «{name}» не найден, используется системный моноширинный");
    }
    discover::default_family(db)
}

// добавленная высота делится поровну над и под строкой
fn spaced(metrics: Metrics, factor: f32) -> Metrics {
    let cell_height = (metrics.cell_height as f32 * factor).round().max(1.0) as u32;
    let extra = (cell_height as i32 - metrics.cell_height as i32) / 2;
    Metrics {
        cell_height,
        baseline: metrics.baseline + extra,
        ..metrics
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
