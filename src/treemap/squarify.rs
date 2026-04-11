#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutItem {
    pub id: usize,
    pub size: f64,
}

#[derive(Debug, Clone)]
pub struct PlacedRect {
    pub id: usize,
    pub rect: Rect,
}

pub fn layout(items: &[LayoutItem], bounds: &Rect) -> Vec<PlacedRect> {
    if items.is_empty() || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return vec![];
    }
    let total_size: f64 = items.iter().map(|i| i.size).sum();
    if total_size <= 0.0 {
        return vec![];
    }
    let mut sorted: Vec<LayoutItem> = items.to_vec();
    sorted.sort_by(|a, b| b.size.partial_cmp(&a.size).unwrap());
    let mut result = Vec::with_capacity(sorted.len());
    let mut remaining = *bounds;
    let area = bounds.w * bounds.h;
    let scale = area / total_size;
    let scaled: Vec<LayoutItem> = sorted
        .iter()
        .map(|item| LayoutItem { id: item.id, size: item.size * scale })
        .collect();
    squarify_recursive(&scaled, &mut remaining, &mut result);
    result
}

fn squarify_recursive(items: &[LayoutItem], bounds: &mut Rect, result: &mut Vec<PlacedRect>) {
    if items.is_empty() || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return;
    }
    if items.len() == 1 {
        result.push(PlacedRect { id: items[0].id, rect: *bounds });
        return;
    }
    let short_side = bounds.w.min(bounds.h);
    let mut row: Vec<&LayoutItem> = vec![];
    let mut row_area = 0.0;
    let mut best_worst_aspect = f64::MAX;
    let mut best_row_len = 1;
    for item in items {
        row.push(item);
        row_area += item.size;
        let worst = worst_aspect_ratio(&row, row_area, short_side);
        if worst <= best_worst_aspect {
            best_worst_aspect = worst;
            best_row_len = row.len();
        } else {
            break;
        }
    }
    let row_items = &items[..best_row_len];
    let row_total: f64 = row_items.iter().map(|i| i.size).sum();
    layout_row(row_items, row_total, bounds, result);
    let remaining = &items[best_row_len..];
    squarify_recursive(remaining, bounds, result);
}

fn worst_aspect_ratio(row: &[&LayoutItem], row_area: f64, short_side: f64) -> f64 {
    if short_side <= 0.0 || row_area <= 0.0 {
        return f64::MAX;
    }
    let s2 = short_side * short_side;
    let mut worst = 0.0_f64;
    for item in row {
        let r = if item.size > 0.0 {
            let a = (s2 * item.size) / (row_area * row_area);
            let b = (row_area * row_area) / (s2 * item.size);
            a.max(b)
        } else {
            f64::MAX
        };
        worst = worst.max(r);
    }
    worst
}

fn layout_row(
    items: &[LayoutItem],
    row_total: f64,
    bounds: &mut Rect,
    result: &mut Vec<PlacedRect>,
) {
    if row_total <= 0.0 {
        return;
    }
    let horizontal = bounds.w >= bounds.h;
    if horizontal {
        let row_width = row_total / bounds.h;
        let mut y = bounds.y;
        for item in items {
            let h = if row_width > 0.0 { item.size / row_width } else { 0.0 };
            result.push(PlacedRect {
                id: item.id,
                rect: Rect { x: bounds.x, y, w: row_width, h },
            });
            y += h;
        }
        bounds.x += row_width;
        bounds.w -= row_width;
    } else {
        let row_height = row_total / bounds.w;
        let mut x = bounds.x;
        for item in items {
            let w = if row_height > 0.0 { item.size / row_height } else { 0.0 };
            result.push(PlacedRect {
                id: item.id,
                rect: Rect { x, y: bounds.y, w, h: row_height },
            });
            x += w;
        }
        bounds.y += row_height;
        bounds.h -= row_height;
    }
}
