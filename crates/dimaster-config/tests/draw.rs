//! `draw.lua`: разбор фигур и помощники.

use dimaster_config::{Plugins, Shape, Snapshot};

const SCRIPT: &str = "dimaster.draw(function(info)
  return {
    { kind = 'rect', x = -10, y = 5, width = tonumber(info.width), height = 1e30 },
    dimaster.circle { x = 20, y = 20, r = 5, color = '#ff0000' },
    dimaster.graph { x = 0, y = 0, width = 100, height = 10, values = { 0, 5, 10 }, max = 10 },
    { kind = 'unknown' }, 7,
  }
end)";

fn drawn() -> Vec<Shape> {
    let directory = std::env::temp_dir().join(format!("dimaster-draw-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("draw.lua"), SCRIPT).unwrap();
    let (plugins, errors) = Plugins::load(&directory).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let facts = [("width".to_owned(), "800".to_owned())];
    let shapes = plugins.draw(&facts, Snapshot::default());
    std::fs::remove_dir_all(directory).unwrap();
    shapes
}

#[test]
fn shapes_helpers_and_limits() {
    let shapes = drawn();
    assert_eq!(shapes.len(), 4, "{shapes:?}");
    assert!(matches!(
        shapes[0],
        Shape::Rect {
            size: (800.0, 20_000.0),
            ..
        }
    ));
    assert!(matches!(
        shapes[1],
        Shape::Rect {
            at: (15.0, 15.0),
            radius: 5,
            ..
        }
    ));
    assert!(matches!(
        shapes[3],
        Shape::Line {
            from: (50.0, 5.0),
            to: (100.0, 0.0),
            ..
        }
    ));
}
