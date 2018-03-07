

/// Chain id for buffers.
pub type BufferChainId = ChainId<(BufferState, BufferUsage)>;

/// `Link` type for buffers.
pub type BufferLink = Link<BufferState, BufferUsage>;

/// `Chain` type for buffers.
pub type BufferChain<S, W> = Chain<BufferState, BufferUsage, (), S, W>;