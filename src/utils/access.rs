use hal::buffer::Access as BufferAccess;
use hal::image::Access as ImageAccess;
use hal::pso::PipelineStage;

// Access type
pub trait Access: Copy {
    fn none() -> Self;
    fn all() -> Self;
    fn is_read(&self) -> bool;
    fn is_write(&self) -> bool;
    fn supported_pipeline_stages(&self) -> PipelineStage;
}

impl Access for ImageAccess {
    fn none() -> Self {
        Self::empty()
    }
    fn all() -> Self {
        Self::all()
    }
    fn is_write(&self) -> bool {
        self.contains(Self::COLOR_ATTACHMENT_WRITE)
            || self.contains(Self::DEPTH_STENCIL_ATTACHMENT_WRITE)
            || self.contains(Self::TRANSFER_WRITE) || self.contains(Self::SHADER_WRITE)
            || self.contains(Self::HOST_WRITE) || self.contains(Self::MEMORY_WRITE)
    }

    fn is_read(&self) -> bool {
        self.contains(Self::COLOR_ATTACHMENT_READ)
            || self.contains(Self::DEPTH_STENCIL_ATTACHMENT_READ)
            || self.contains(Self::TRANSFER_READ) || self.contains(Self::SHADER_READ)
            || self.contains(Self::HOST_READ) || self.contains(Self::MEMORY_READ)
            || self.contains(Self::INPUT_ATTACHMENT_READ)
    }

    fn supported_pipeline_stages(&self) -> PipelineStage {
        type PS = PipelineStage;

        match *self {
            Self::COLOR_ATTACHMENT_READ | Self::COLOR_ATTACHMENT_WRITE => {
                PS::COLOR_ATTACHMENT_OUTPUT
            }
            Self::TRANSFER_READ | Self::TRANSFER_WRITE => PS::TRANSFER,
            Self::SHADER_READ | Self::SHADER_WRITE => {
                PS::VERTEX_SHADER | PS::GEOMETRY_SHADER | PS::FRAGMENT_SHADER | PS::COMPUTE_SHADER
            }
            Self::DEPTH_STENCIL_ATTACHMENT_READ | Self::DEPTH_STENCIL_ATTACHMENT_WRITE => {
                PS::EARLY_FRAGMENT_TESTS | PS::LATE_FRAGMENT_TESTS
            }
            Self::HOST_READ | Self::HOST_WRITE => PS::HOST,
            Self::MEMORY_READ | Self::MEMORY_WRITE => PS::empty(),
            Self::INPUT_ATTACHMENT_READ => PS::FRAGMENT_SHADER,
            _ => panic!("Only one bit must be set"),
        }
    }
}

impl Access for BufferAccess {
    fn none() -> Self {
        Self::empty()
    }
    fn all() -> Self {
        Self::all()
    }
    fn is_write(&self) -> bool {
        self.contains(Self::TRANSFER_WRITE) || self.contains(Self::SHADER_WRITE)
            || self.contains(Self::HOST_WRITE) || self.contains(Self::MEMORY_WRITE)
    }

    fn is_read(&self) -> bool {
        self.contains(Self::TRANSFER_READ) || self.contains(Self::SHADER_READ)
            || self.contains(Self::HOST_READ) || self.contains(Self::MEMORY_READ)
            || self.contains(Self::INDEX_BUFFER_READ)
            || self.contains(Self::VERTEX_BUFFER_READ)
            || self.contains(Self::INDIRECT_COMMAND_READ)
            || self.contains(Self::CONSTANT_BUFFER_READ)
    }

    fn supported_pipeline_stages(&self) -> PipelineStage {
        type PS = PipelineStage;

        match *self {
            Self::TRANSFER_READ | Self::TRANSFER_WRITE => PS::TRANSFER,
            Self::INDEX_BUFFER_READ | Self::VERTEX_BUFFER_READ => PS::VERTEX_INPUT,
            Self::INDIRECT_COMMAND_READ => PS::DRAW_INDIRECT,
            Self::CONSTANT_BUFFER_READ | Self::SHADER_READ | Self::SHADER_WRITE => {
                PS::VERTEX_SHADER | PS::GEOMETRY_SHADER | PS::FRAGMENT_SHADER | PS::COMPUTE_SHADER
            }
            Self::HOST_READ | Self::HOST_WRITE => PS::HOST,
            Self::MEMORY_READ | Self::MEMORY_WRITE => PS::empty(),
            _ => panic!("Only one bit must be set"),
        }
    }
}
