use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitOrAssign, Index, IndexMut, Range};

use hal::Backend;
use hal::buffer::{State as BufferState};
use hal::command::{CommandBuffer};
use hal::memory::{Barrier, Dependencies};
use hal::pso::PipelineStage;
use hal::queue::{Transfer, QueueFamilyId, Supports};

use utils::{Access, State};

/// Unique identifier of the queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct QueueId {
    pub index: usize,
    pub family: QueueFamilyId,
}

/// Unique identifier for resource dependency chain.
/// Multiple resource can be associated with single chain
/// if all passes uses them the same way.
/// Chain id uses marker type so chain ids for buffers and images are different.
#[derive(Derivative)]
#[derivative(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ChainId<T>(usize, PhantomData<T>);
impl<T> ChainId<T> {
    /// Make new chain id.
    pub fn new(index: usize) -> Self {
        ChainId(index, PhantomData)
    }

    /// Get index value.
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Piece of `Chain` associated with single pass.
/// Specify usage and state of the resource.
/// Those links can be combined to form chains.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Link<T, U> {
    pub id: ChainId<T>,
    pub stages: PipelineStage,
    pub state: T,
    pub usage: U,
}

/// All links pass defines.
#[derive(Clone, Debug)]
pub struct PassLinks<T, U> {
    /// queue identifier on which commands recorded by the pass will be executed.
    pub queue: QueueId,
    /// List of links this pass uses.
    pub links: Vec<Link<T, U>>,
}

#[derive(Clone, Copy, Debug)]
struct Acquire;
#[derive(Clone, Copy, Debug)]
struct Release;

trait Semantics {
    fn src_dst(this: QueueFamilyId, other: QueueFamilyId) -> (QueueFamilyId, QueueFamilyId);
}

impl Semantics for Acquire {
    fn src_dst(this: QueueFamilyId, other: QueueFamilyId) -> (QueueFamilyId, QueueFamilyId) {
        (other, this)
    }
}

impl Semantics for Release {
    fn src_dst(this: QueueFamilyId, other: QueueFamilyId) -> (QueueFamilyId, QueueFamilyId) {
        (this, other)
    }
}

/// Link acquire operation.
/// Defines what synchronization operation is required before pass can record commands.
#[derive(Derivative)]
#[derivative(Clone, Debug)]
enum LinkSync<T, S, M> {
    /// No transition required.
    None(M),

    /// Insert pipeline barrier.
    /// Or setup dependency and transition via render-pass info for attachments.
    Barrier {
        states: Range<T>,
        stages: Range<PipelineStage>,
    },

    /// Wait of semaphore.
    Semaphore { semaphore: S },

    /// Wait semaphore and insert barrier.
    BarrierSemaphore {
        semaphore: S,
        states: Range<T>,
        stages: Range<PipelineStage>,
    },

    /// Perform ownership transfer.
    Transfer {
        semaphore: S,
        states: Range<T>,
        stages: Range<PipelineStage>,
        other: QueueId,
    },
}

impl<T, S, B> LinkSync<T, S, B> {
    fn is_none(&self) -> bool {
        match *self {
            LinkSync::None(_) => true,
            _ => false,
        }
    }
}

impl<T, W> LinkSync<T, W, Acquire> {
    /// Report what semaphore should be waited before executing commands of the link.
    fn wait<B>(&self) -> Option<&W>
    where
        B: Backend,
    {
        match *self {
            LinkSync::None(_) => None,
            LinkSync::Semaphore { ref semaphore }
            | LinkSync::BarrierSemaphore { ref semaphore, .. } => Some(semaphore),
        }
    }
}

impl<T, S> LinkSync<T, S, Release> {
    /// Report what semaphore should be signaled after executing commands of the link.
    fn signal<B>(&self) -> Option<&S>
    where
        B: Backend,
    {
        match *self {
            LinkSync::None(_) => None,
            LinkSync::Semaphore { ref semaphore }
            | LinkSync::BarrierSemaphore { ref semaphore, .. } => Some(semaphore),
        }
    }
}

impl<T, S, M> LinkSync<T, S, M>
where
    S: Semantics,
{
    /// Insert barrier if required before recording commands for the link.
    fn barrier<B, C, R>(
        &self,
        this: QueueId,
        commands: &CommandBuffer<B, C>,
        buffers: Option<&[&R]>,
    ) where
        B: Backend,
        C: Supports<Transfer>,
        T: IntoBarrier<R>,
    {
        let (states, stages, (src, dst)) = match *self {
            LinkSync::None(_) | LinkSync::Semaphore { .. } => {
                return;
            }
            LinkSync::Barrier { states, stages } => (states, stages, (this.family, this.family)),
            LinkSync::BarrierSemaphore {
                states,
                stages,
                ..
            } => (states, stages, (this.family, this.family)),
            LinkSync::Transfer {
                states,
                stages,
                other,
                ..
            } => (states, stages, S::src_dst(this.family, other.family))
        };
        if src != dst {
            unimplemented!();
        }
        match buffers {
            Some(buffers) => {
                commands.pipeline_barrier(
                    stages,
                    Dependencies::empty(),
                    buffers
                        .iter()
                        .map(|&target| T::IntoBarrier(states, target)),
                );
            }
            None => {
                commands.pipeline_barrier(
                    stages,
                    Dependencies::empty(),
                    Some(Barrier::AllBuffers(states)),
                );
            }
        }
    }
}

/// Link of the fully formed chain.
#[derive(Clone, Debug)]
pub struct ChainLink<T, S, W> {
    queue: QueueId,
    stages: PipelineStage,
    state: T,
    merged_state: T,
    merged_stages: PipelineStage,
    acquire: LinkSync<T, W, Acquire>,
    release: LinkSync<T, S, Release>,
}

/// Fully formed chain.
#[derive(Clone, Debug)]
pub struct Chain<T, U, I, S, W = S> {
    pub usage: U,
    pub init: I,
    pub links: Vec<Option<ChainLink<T, S, W>>>,
}

impl<T, U, I, S, W> Chain<T, U, I, S, W> {
    /// Build chain from links defined by passes.
    fn build<P, F>(
        id: ChainId<T>,
        mut usage: U,
        init: I,
        passes: P,
        mut new_semaphore: F,
    ) -> Option<Self>
    where
        P: IntoIterator,
        P::Item: Borrow<PassLinks<T, U>>,
        T: State,
        U: BitOrAssign + Copy,
        F: FnMut() -> (S, W),
    {
        let mut links: Vec<Option<ChainLink<T, S, W>>> = Vec::new();

        // Walk over passes
        for pass in passes {
            let pass = pass.borrow();
            // Collect links from passes.
            links.push(pass.links.iter().find(|link| link.id == id).map(|link| {
                usage |= link.usage;
                ChainLink {
                    queue: pass.queue,
                    stages: link.stages,
                    state: link.state,
                    merged_state: link.state,
                    merged_stages: link.stages,
                    acquire: LinkSync::None(Acquire),
                    release: LinkSync::None(Release),
                }
            }));
        }

        let count = links.len();

        // Walk over all links twice and merge states of compatible sub-chains
        for index in 0..(count * 2) {
            let index = index % count;
            let (before, link_after) = links.split_at_mut(index);
            let (link, after) = link_after.split_first_mut().unwrap();

            // Skip non-existing
            let link = if let Some(link) = link.as_mut() {
                link
            } else {
                continue;
            };

            // Get next existing link
            if let Some(next) = after
                .iter_mut()
                .chain(before.iter_mut())
                .filter_map(Option::as_mut)
                .next()
            {
                match link.state.merge(next.state) {
                    Some(state) if link.queue == next.queue => {
                        link.merged_state = state;
                        next.merged_state = state;
                        let stages = link.stages | next.stages;
                        link.merged_stages = stages;
                        next.merged_stages = stages;
                    }
                    _ => {}
                }
            } else {
                // No other links
                break;
            }
        }

        for index in 0..count {
            let (before, link_after) = links.split_at_mut(index);
            let (link, after) = link_after.split_first_mut().unwrap();

            // Skip non-existing
            let link = if let Some(link) = link.as_mut() {
                link
            } else {
                continue;
            };

            if let Some(next) = after
                .iter_mut()
                .chain(before.iter_mut())
                .filter_map(Option::as_mut)
                .next()
            {
                debug_assert!(link.release.is_none());
                debug_assert!(next.acquire.is_none());

                let states = if next.state.access().is_read() {
                    link.merged_state
                } else {
                    link.merged_state.discard_content()
                }..next.merged_state;

                let stages = link.merged_stages..next.merged_stages;

                match link.state.merge(next.state) {
                    Some(state) if link.queue == next.queue => {
                        // Verify that they are merged properly
                    }
                    _ if link.queue == next.queue => {
                        // Incompatible states on same queue. Insert barrier.
                        link.release = LinkSync::Barrier { states, stages };
                    }
                    _ if link.queue.family == next.queue.family => {
                        let (signal, wait) = new_semaphore();
                        states.start = states.start.with_no_access();
                        states.end = states.end.with_no_access();
                        // Different queues from same family
                        if states.start == states.end {
                            // Signal + Wait. Barrier isn't required.
                            link.release = LinkSync::Semaphore { semaphore: signal };
                            next.acquire = LinkSync::Semaphore { semaphore: wait };
                        } else {
                            // Barrier + Signal + Wait
                            link.release = LinkSync::BarrierSemaphore {
                                semaphore: signal,
                                states,
                                stages,
                            };

                            next.acquire = LinkSync::Semaphore { semaphore: wait };
                        }
                    }
                    _ => {
                        let (signal, wait) = new_semaphore();

                        states.start = states.start.with_no_access();
                        states.end = states.end.with_no_access();

                        // Different queues from different family
                        // Barrier + Signal + Wait + Barrier with ownership transfer.
                        link.release = LinkSync::Transfer {
                            semaphore: signal,
                            states,
                            stages,
                            other: next.queue,
                        };

                        next.acquire = LinkSync::Transfer {
                            semaphore: wait,
                            states,
                            stages,
                            other: link.queue,
                        };
                    }
                }
            } else {
                // No other links
                break;
            }
        }
        if links.iter().all(Option::is_none) {
            None
        } else {
            Some(Chain { links, usage, init })
        }
    }

    pub fn link(&self, index: usize) -> &ChainLink<T, S, W> {
        self.links[index].as_ref().unwrap()
    }

    pub fn link_mut(&mut self, index: usize) -> &mut ChainLink<T, S, W> {
        self.links[index].as_mut().unwrap()
    }

    pub fn prev(&self, index: usize) -> Option<&ChainLink<T, S, W>> {
        self.links[..index]
            .iter()
            .rev()
            .filter_map(Option::as_ref)
            .next()
    }

    pub fn prev_mut(&mut self, index: usize) -> Option<&mut ChainLink<T, S, W>> {
        self.links[..index]
            .iter_mut()
            .rev()
            .filter_map(Option::as_mut)
            .next()
    }

    pub fn next(&self, index: usize) -> Option<&ChainLink<T, S, W>> {
        self.links[index + 1..]
            .iter()
            .filter_map(Option::as_ref)
            .next()
    }

    pub fn next_mut(&mut self, index: usize) -> Option<&mut ChainLink<T, S, W>> {
        self.links[index + 1..]
            .iter_mut()
            .filter_map(Option::as_mut)
            .next()
    }

    pub fn first(&self) -> &ChainLink<T, S, W> {
        self.links.iter().filter_map(Option::as_ref).next().unwrap()
    }

    pub fn first_mut(&mut self) -> &mut ChainLink<T, S, W> {
        self.links
            .iter_mut()
            .filter_map(Option::as_mut)
            .next()
            .unwrap()
    }

    pub fn last(&self) -> &ChainLink<T, S, W> {
        self.links
            .iter()
            .rev()
            .filter_map(Option::as_ref)
            .next()
            .unwrap()
    }

    pub fn last_mut(&mut self) -> &mut ChainLink<T, S, W> {
        self.links
            .iter_mut()
            .rev()
            .filter_map(Option::as_mut)
            .next()
            .unwrap()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GraphChains<T, U, I, S, W> {
    chains: Vec<Chain<T, U, I, S, W>>,
}

impl<T, U, I, S, W> GraphChains<T, U, I, S, W>
where
    T: State,
    U: BitOrAssign + Copy,
{
    pub(crate) fn new(
        count: usize,
        init: I,
        usage: U,
        links: &[PassLinks<T, U>],
    ) -> GraphChains<T, U, I, S, W>
    where
        I: Copy,
    {
        GraphChains {
            chains: (0..count)
                .map(|i| Chain::build(ChainId::new(i), usage, init, links, || unimplemented!()))
                .collect(),
        }
    }

    pub fn chain(&self, id: ChainId<T>) -> &Chain<T, U, I, S, W> {
        &self.chains[id.0]
    }

    pub fn chain_mut(&mut self, id: ChainId<T>) -> &mut Chain<T, U, I, S, W> {
        &mut self.chains[id.0]
    }
}

impl<T, U, I, S, W> Index<ChainId<T>> for GraphChains<T, U, I, S, W>
where
    T: State,
    U: BitOrAssign + Copy,
{
    type Output = Chain<T, U, I, S, W>;
    fn index(&self, index: ChainId<T>) -> &Chain<T, U, I, S, W> {
        self.chain(index)
    }
}

impl<T, U, I, S, W> IndexMut<ChainId<T>> for GraphChains<T, U, I, S, W>
where
    T: State,
    U: BitOrAssign + Copy,
{
    fn index_mut(&mut self, index: ChainId<T>) -> &mut Chain<T, U, I, S, W> {
        self.chain_mut(index)
    }
}
