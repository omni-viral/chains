use hal::image::ImageLayout;

pub fn merge_image_layouts<L>(layout: ImageLayout, layouts: L) -> ImageLayout
where
    L: IntoIterator<Item = ImageLayout>,
{
    layouts
        .into_iter()
        .fold(layout, |layout, next| common_image_layout(layout, next))
}

pub fn common_image_layout(left: ImageLayout, right: ImageLayout) -> ImageLayout {
    match (left, right) {
        (x, y) if x == y => x,
        (ImageLayout::Present, _) | (_, ImageLayout::Present) => {
            panic!("Present layout is unexpected here")
        }
        (ImageLayout::ShaderReadOnlyOptimal, ImageLayout::DepthStencilReadOnlyOptimal)
        | (ImageLayout::DepthStencilReadOnlyOptimal, ImageLayout::ShaderReadOnlyOptimal) => {
            ImageLayout::DepthStencilReadOnlyOptimal
        }
        (_, _) => ImageLayout::General,
    }
}
