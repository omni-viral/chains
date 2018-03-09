use hal::pso::PipelineStage;

use chain::chain::ChainId;
use resource::Resource;
use queue::QueueId;

/// Piece of `Chain` associated with single pass.
/// Specify usage and state of the resource.
/// Those links can be combined to form chains.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Link<R: Resource> {
    pub id: ChainId<R>,
    pub stages: PipelineStage,
    pub access: R::Access,
    pub layout: R::Layout,
    pub usage: R::Usage,
}

/// All links pass defines.
#[derive(Clone, Debug)]
pub struct PassLinks<R: Resource> {
    /// queue identifier on which commands recorded by the pass will be executed.
    pub queue: QueueId,
    /// List of links this pass uses.
    pub links: Vec<Link<R>>,
}
