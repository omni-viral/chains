use hal::command::ClearValue;
use hal::pass::AttachmentLoadOp;

/// Image initialization.
/// Either image content is loaded. Typical for textures
/// Cleared. Typical for attachments.
/// Or just discarded. Also typical for attachments.
#[derive(Clone, Copy, Debug)]
pub enum ImageInit {
    Load,
    DontCare,
    Clear(ClearValue),
}

impl ImageInit {
    /// Get attachment load operation for the image.
    pub fn load_op(&self) -> AttachmentLoadOp {
        match *self {
            ImageInit::Clear(_) => AttachmentLoadOp::Clear,
            ImageInit::Load => AttachmentLoadOp::Load,
            ImageInit::DontCare => AttachmentLoadOp::DontCare,
        }
    }

    /// Check if image content is discarded on start of each frame. i.e. it is not `Load`
    pub fn discard(&self) -> bool {
        match *self {
            ImageInit::Load => false,
            _ => true,
        }
    }

    /// Get `Some` clear value or `None`.
    pub fn clear_value(&self) -> Option<ClearValue> {
        match *self {
            ImageInit::Clear(value) => Some(value),
            _ => None,
        }
    }
}
