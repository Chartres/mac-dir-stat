use mac_dir_stat::treemap::squarify::{layout, LayoutItem, Rect};

#[test]
fn test_single_item() {
    let items = vec![LayoutItem { id: 0, size: 100.0 }];
    let bounds = Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
    let rects = layout(&items, &bounds);
    assert_eq!(rects.len(), 1);
    assert!((rects[0].rect.w - 800.0).abs() < 0.01);
    assert!((rects[0].rect.h - 600.0).abs() < 0.01);
}

#[test]
fn test_two_equal_items() {
    let items = vec![
        LayoutItem { id: 0, size: 100.0 },
        LayoutItem { id: 1, size: 100.0 },
    ];
    let bounds = Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
    let rects = layout(&items, &bounds);
    assert_eq!(rects.len(), 2);
    let total_area: f64 = rects.iter().map(|r| r.rect.w * r.rect.h).sum();
    assert!((total_area - 800.0 * 600.0).abs() < 1.0);
    let area0 = rects[0].rect.w * rects[0].rect.h;
    let area1 = rects[1].rect.w * rects[1].rect.h;
    assert!((area0 - area1).abs() < 1.0);
}

#[test]
fn test_areas_proportional_to_sizes() {
    let items = vec![
        LayoutItem { id: 0, size: 600.0 },
        LayoutItem { id: 1, size: 300.0 },
        LayoutItem { id: 2, size: 100.0 },
    ];
    let bounds = Rect { x: 0.0, y: 0.0, w: 1000.0, h: 1000.0 };
    let rects = layout(&items, &bounds);
    assert_eq!(rects.len(), 3);
    let areas: Vec<f64> = rects.iter().map(|r| r.rect.w * r.rect.h).collect();
    let total: f64 = areas.iter().sum();
    assert!((areas[0] / total - 0.6).abs() < 0.01);
    assert!((areas[1] / total - 0.3).abs() < 0.01);
    assert!((areas[2] / total - 0.1).abs() < 0.01);
}

#[test]
fn test_no_overlap() {
    let items: Vec<LayoutItem> = (0..20)
        .map(|i| LayoutItem { id: i, size: (20 - i) as f64 * 100.0 })
        .collect();
    let bounds = Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
    let rects = layout(&items, &bounds);
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate() {
            if i >= j { continue; }
            let overlap_x = a.rect.x < b.rect.x + b.rect.w && a.rect.x + a.rect.w > b.rect.x;
            let overlap_y = a.rect.y < b.rect.y + b.rect.h && a.rect.y + a.rect.h > b.rect.y;
            if overlap_x && overlap_y {
                let overlap_w = (a.rect.x + a.rect.w).min(b.rect.x + b.rect.w) - a.rect.x.max(b.rect.x);
                let overlap_h = (a.rect.y + a.rect.h).min(b.rect.y + b.rect.h) - a.rect.y.max(b.rect.y);
                assert!(overlap_w < 0.1 || overlap_h < 0.1, "Rects {} and {} overlap by {}x{}", i, j, overlap_w, overlap_h);
            }
        }
    }
}

#[test]
fn test_all_within_bounds() {
    let items: Vec<LayoutItem> = (0..50)
        .map(|i| LayoutItem { id: i, size: (50 - i) as f64 * 10.0 + 1.0 })
        .collect();
    let bounds = Rect { x: 10.0, y: 20.0, w: 780.0, h: 560.0 };
    let rects = layout(&items, &bounds);
    for r in &rects {
        assert!(r.rect.x >= bounds.x - 0.01);
        assert!(r.rect.y >= bounds.y - 0.01);
        assert!(r.rect.x + r.rect.w <= bounds.x + bounds.w + 0.01);
        assert!(r.rect.y + r.rect.h <= bounds.y + bounds.h + 0.01);
    }
}

#[test]
fn test_aspect_ratios_reasonable() {
    let items: Vec<LayoutItem> = (0..10)
        .map(|i| LayoutItem { id: i, size: (10 - i) as f64 * 50.0 + 10.0 })
        .collect();
    let bounds = Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
    let rects = layout(&items, &bounds);
    for r in &rects {
        let aspect = if r.rect.w > r.rect.h { r.rect.w / r.rect.h } else { r.rect.h / r.rect.w };
        assert!(aspect < 5.0, "Bad aspect ratio {} for rect {:?}", aspect, r);
    }
}

#[test]
fn test_empty_input() {
    let items: Vec<LayoutItem> = vec![];
    let bounds = Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
    let rects = layout(&items, &bounds);
    assert!(rects.is_empty());
}
