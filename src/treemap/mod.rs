pub mod squarify;
pub mod color;

use crate::scanner::tree::NodeId;
use crate::treemap::squarify::Rect;
use crate::treemap::color::Color;

#[derive(Debug, Clone)]
pub struct ColoredRect {
    pub node_id: NodeId,
    pub rect: Rect,
    pub color_start: Color,
    pub color_end: Color,
}
