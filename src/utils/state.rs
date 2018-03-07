use std::ops::Range;
use hal::Backend;
use hal::buffer::{Access as BufferAccess, State as BufferState};
use hal::image::{Access as ImageAccess, State as ImageState, ImageLayout};
use hal::memory::Barrier;
use utils::access::Access;
use utils::layout::common_image_layout;

/// Resource state trait.
/// It is implemented for `gfx_hal::buffer::State` and `gfx_hal::image::State`.
pub trait State: Copy + Eq + Sized {
    type Access: Access;

    /// Get access of the state.
    fn access(self) -> Self::Access;

    /// Transform state transition so it may discard content.
    fn discard_content(self) -> Self {
        self
    }

    /// Replace access of the state
    fn replace_access(self, access: Self::Access) -> Self;

    /// Try merge two states.
    /// return `None` if they can't be merged.
    fn merge(self, rhs: Self) -> Option<Self>;

    /// Check if state is compatible with another.
    /// They are mergeable and merge yields same state.
    fn compatible(self, rhs: Self) -> bool {
        self.merge(rhs) == Some(self)
    }

    /// Add all access types.
    fn with_all_access(self) -> Self {
        self.replace_access(Self::Access::all())
    }

    /// Remove all access types.
    fn with_no_access(self) -> Self {
        self.replace_access(Self::Access::none())
    }
}

impl State for BufferState {
    type Access = BufferAccess;

    /// Get access of the state.
    fn access(self) -> BufferAccess {
        self
    }

    fn merge(self, rhs: Self) -> Option<Self> {
        if self.is_read() && rhs.is_read() {
            Some(self | rhs)
        } else {
            None
        }
    }

    fn replace_access(self, access: BufferAccess) -> Self {
        access
    }
}


impl State for ImageState {
    type Access = ImageAccess;

    /// Get access of the state.
    fn access(self) -> ImageAccess {
        self.0
    }

    fn discard_content(self) -> Self {
        (self.0, ImageLayout::Undefined)
    }

    fn merge(self, rhs: Self) -> Option<Self> {
        if self.0.is_read() && rhs.0.is_read() {
            Some((self.0 | rhs.0, common_image_layout(self.1, rhs.1)))
        } else {
            None
        }
    }

    fn replace_access(self, access: ImageAccess) -> Self {
        (access, self.1)
    }
}


pub trait BigBarrier<B>: State {
    fn big_barrier<'a>(states: Range<Self>) -> Barrier<'a, B>;
}

impl<B> BigBarrier<B> for BufferState
where
    B: Backend,
{
    fn big_barrier<'a>(states: Range<BufferState>) -> Barrier<'a, B> {
        Barrier::AllBuffers(states)
    }
}


impl<B> BigBarrier<B> for ImageState
where
    B: Backend,
{
    fn big_barrier<'a>(states: Range<ImageState>) -> Barrier<'a, B> {
        Barrier::AllImages(states.0)
    }
}


pub trait TargetBarrier<B, R>: BigBarrier<B> {
    fn target_barrier<'a>(states: Range<Self>, target: &'a R) -> Barrier<'a, B>;
}


impl<B> TargetBarrier<B, B::Buffer> for BufferState
where
    B: Backend,
{
    fn target_barrier<'a>(states: Range<BufferState>, target &'a B::Buffer) -> Barrier<'a, B> {
        Barrier::Buffer {
            states,
            target,
        }
    }
}

impl<B> TargetBarrier<B, B::Image> for ImageState
where
    B: Backend,
{
    fn target_barrier<'a>(states: Range<ImageState>, target &'a B::Image) -> Barrier<'a, B> {
        Barrier::Image {
            states,
            target,
        }
    }
}
