use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Tree};

const SHIELD_ON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <path d="M32 4L8 14v16c0 14.8 10.24 28.64 24 32 13.76-3.36 24-17.2 24-32V14L32 4z" fill="#4ade80" stroke="#22c55e" stroke-width="1.5"/>
  <path d="M24 33l6 6 12-12" fill="none" stroke="#ffffff" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"##;

const SHIELD_OFF_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <path d="M32 4L8 14v16c0 14.8 10.24 28.64 24 32 13.76-3.36 24-17.2 24-32V14L32 4z" fill="none" stroke="#64748b" stroke-width="2.5"/>
  <path d="M24 24l16 16M40 24l-16 16" fill="none" stroke="#64748b" stroke-width="4" stroke-linecap="round"/>
</svg>"##;

pub fn shield_on_svg() -> &'static str {
    SHIELD_ON_SVG
}

pub fn shield_off_svg() -> &'static str {
    SHIELD_OFF_SVG
}

/// Render an SVG string to ARGB32 pixel data (network byte order) for ksni::Icon
pub fn render_svg_to_argb(svg: &str, size: u32) -> Option<(i32, i32, Vec<u8>)> {
    let tree = Tree::from_str(svg, &Options::default()).ok()?;
    let mut pixmap = Pixmap::new(size, size)?;

    let scale_x = size as f32 / tree.size().width();
    let scale_y = size as f32 / tree.size().height();
    let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny_skia gives us premultiplied RGBA, ksni wants ARGB32 in network byte order
    let rgba = pixmap.data();
    let mut argb = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
        argb.push(a);
        argb.push(r);
        argb.push(g);
        argb.push(b);
    }

    Some((size as i32, size as i32, argb))
}
