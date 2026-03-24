
/// lightwight configuration struct for WGPU pipelines
#[derive(Clone)]
pub struct RenderPipelineConfig {
    /// unique identifier for the pipeline
    pub name: String,
    /// the path to the pipeline's shader source file
    pub path: String,
    /// the name of the entry function into the vertex stage 
    pub vert_main: String,
    /// the name of the entry function into the fragment stage
    pub frag_main: String,
}

pub enum Pipeline {
    /// Simple 2D colored sprite rendering pipeline
    ColoredSprite,
}

impl Pipeline {

    /// Returns a RenderPipelineConfig corresponding to the pipeline preset variant
    pub fn instance(&self) -> RenderPipelineConfig {
        return match *self {
            Pipeline::ColoredSprite => RenderPipelineConfig {
                name: "shader".into(),
                path: "assets/shaders/shader.wgsl".into(),
                vert_main: "vs_main".into(),
                frag_main: "fs_main".into(),
            },
        }
    }
}