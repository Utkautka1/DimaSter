//! Вписывание картинки в рамку из ячеек (`fit=` собственного протокола).
//!
//! Считает терминал, а не программа: только он знает размер ячейки в пикселях.

/// Занятые ячейки и вырезаемый кусок картинки `[x, y, ширина, высота]`.
pub(crate) type Fitted = ((usize, usize), Option<[u32; 4]>);

// центральный кусок картинки с пропорциями рамки
fn cover(image: (usize, usize), frame: (usize, usize)) -> [u32; 4] {
    let (iw, ih, fw, fh) = (
        image.0 as u64,
        image.1 as u64,
        frame.0 as u64,
        frame.1 as u64,
    );
    let (w, h) = match iw * fh > ih * fw {
        true => ((ih * fw / fh).max(1), ih),
        false => (iw, (iw * fh / fw).max(1)),
    };
    [(iw - w) / 2, (ih - h) / 2, w, h].map(|v| v as u32)
}

// вся картинка целиком: одна из сторон рамки уменьшается
fn contain(image: (usize, usize), cells: (usize, usize), cell: (usize, usize)) -> (usize, usize) {
    let (width, height) = (cells.0 * cell.0, cells.1 * cell.1);
    match image.0 * height > image.1 * width {
        true => (cells.0, (width * image.1 / image.0).div_ceil(cell.1).max(1)),
        false => (
            (height * image.0 / image.1).div_ceil(cell.0).max(1),
            cells.1,
        ),
    }
}

/// `mode`: `cover`, `contain`; иначе картинка растягивается.
pub(crate) fn fit(
    mode: &str,
    image: (usize, usize),
    cells: (usize, usize),
    cell: (usize, usize),
) -> Fitted {
    let image = (image.0.max(1), image.1.max(1));
    let cells = (cells.0.max(1), cells.1.max(1));
    let frame = (cells.0 * cell.0.max(1), cells.1 * cell.1.max(1));
    match mode {
        "cover" => (cells, Some(cover(image, frame))),
        "contain" => (contain(image, cells, (cell.0.max(1), cell.1.max(1))), None),
        _ => (cells, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_crops_and_contain_shrinks() {
        // рамка 20×10 ячеек по 10×20 пикселей — квадрат
        let wide = fit("cover", (400, 200), (20, 10), (10, 20));
        assert_eq!(wide, ((20, 10), Some([100, 0, 200, 200])));
        let tall = fit("cover", (200, 400), (20, 10), (10, 20));
        assert_eq!(tall.1, Some([0, 100, 200, 200]));
        assert_eq!(fit("contain", (400, 200), (20, 10), (10, 20)).0, (20, 5));
        assert_eq!(fit("", (400, 200), (20, 10), (10, 20)), ((20, 10), None));
    }
}
