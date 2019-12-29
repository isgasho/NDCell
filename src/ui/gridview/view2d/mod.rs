use std::rc::Rc;

mod render;
mod shaders;
mod viewport;
mod zoom;

use super::GridViewTrait;
use crate::automaton::NdProjectedAutomatonTrait;
use crate::automaton::*;
pub use viewport::Viewport2D;
pub use zoom::Zoom2D;

pub struct GridView2D {
    /// Automaton being simulated and displayed.
    pub automaton: ProjectedAutomaton<Dim2D>,
    /// Target viewport.
    pub viewport: Viewport2D,
    /// Viewport that interpolates to the target and is used for drawing.
    pub interpolating_viewport: Viewport2D,
    render_cache: render::RenderCache,
    shaders: render::Shaders,
    vbos: render::VBOs,
    display: Rc<glium::Display>,
}
impl GridView2D {
    pub fn new(display: Rc<glium::Display>, automaton: ProjectedAutomaton<Dim2D>) -> Self {
        Self {
            automaton: ProjectedAutomaton::from(automaton),
            viewport: Default::default(),
            interpolating_viewport: Default::default(),
            render_cache: render::RenderCache::default(),
            shaders: render::Shaders::compile(&*display),
            vbos: render::VBOs::new(&*display),
            display,
        }
    }
    pub fn default(display: Rc<glium::Display>) -> Self {
        Self::new(display, ProjectedAutomaton::default())
    }
    pub fn use_viewport_from(&mut self, other: &Self) {
        self.viewport = other.viewport.clone();
        self.interpolating_viewport = other.interpolating_viewport.clone();
    }
}

impl GridViewTrait for GridView2D {
    fn draw(&mut self, target: &mut glium::Frame) {
        render::draw(self, target);
    }
    fn do_frame(&mut self) {
        const DECAY_CONSTANT: f32 = 4.0;
        if self.interpolating_viewport != self.viewport {
            self.interpolating_viewport = Viewport2D::interpolate(
                &self.interpolating_viewport,
                &self.viewport,
                1.0 / DECAY_CONSTANT,
            );
        }
    }
    fn get_population(&self) -> usize {
        self.automaton.get_population()
    }
    fn get_generation_count(&self) -> usize {
        self.automaton.get_generation_count()
    }
}
