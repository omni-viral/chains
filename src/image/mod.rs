

/// Chain id for images.
pub type ImageChainId = ChainId<(ImageState, ImageUsage)>;

// `Link` type for images.
pub type ImageLink = Link<ImageState, ImageUsage>;

/// `Chain` type for images.
pub type ImageChain<S, W> = Chain<ImageState, ImageUsage, ImageInit, S, W>;



/// Methods specific for image chains.
impl<S, W> ImageChain<S, W> {
    /// Setup additional synchronization for presentable attachment
    pub fn with_presentable_sync(&mut self, acquire: W, from: ImageLayout, release: S, to: ImageLayout) {
        let link = self.first_mut();
        match link.acquire {
            LinkSync::None(Acquire) => {},
            _ => panic!("First link is already synchronized with something"),
        }
        link.acquire = LinkSync::Transfer {
            queue: link.queue,
            semaphore: acquire,
            states: (ImageAccess::empty(), from) .. link.state,
            stages: PipelineStage::BOTTOM_OF_PIPE .. link.stages,
        };
        let link = self.last_mut();
        match link.release {
            LinkSync::None(Release) => {},
            _ => panic!("Last link is already synchronized with something"),
        }
        link.release = LinkSync::Transfer {
            queue: link.queue,
            semaphore: release,
            states: link.state .. (ImageAccess::empty(), to),
            stages: link.stages .. PipelineStage::TOP_OF_PIPE,
        };
    }

    /// Load operation for attachment used in render-pass
    pub fn load_op(&self, index: usize) -> AttachmentLoadOp {
        if self.link(index).state.0.is_read() {
            AttachmentLoadOp::Load
        } else {
            self.init.load_op()
        }
    }

    /// Store operation for attachment used in render-pass
    pub fn store_op(&self, index: usize) -> AttachmentStoreOp {
        if self.links[index + 1..].iter().filter_map(Option::as_ref).any(|link| link.state.0.is_read()) {
            return AttachmentStoreOp::Store;
        } else {
            AttachmentStoreOp::DontCare
        }
    }

    /// 
    pub fn pass_layout_transition(&self, index: usize) -> Range<ImageLayout> {
        let ref link = self.link(index);
        let start = match link.acquire {
            LinkSync::None(Acquire) | LinkSync::Semaphore { .. } => { link.state.1 },
            LinkSync::Transfer { states } => {
                debug_assert_eq!(states.end.1, link.state.1);
                unimplemented!()
            }
        };
    }

    pub fn subpass_layout(&self, index: usize) -> ImageLayout {
        self.link(index).state.1
    }

    pub fn clear_value(&self, index: usize) -> Option<ClearValue> {
        match self.load_op(index) {
            AttachmentLoadOp::Clear => self.init.clear_value(),
            _ => None,
        }
    }
}
