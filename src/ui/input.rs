use glium::glutin::*;
use std::collections::HashSet;
use std::ops::Index;

use super::gridview;
use crate::automaton::Vec2D;

const FALSE_REF: &bool = &false;
const TRUE_REF: &bool = &true;

/// A struct tracking miscellaneous stateful things relating input, such as
/// whether any given key is pressed.
#[derive(Default)]
pub struct InputState {
    scancodes: HashSet<u32>,
    virtual_keycodes: HashSet<VirtualKeyCode>,
    /// Whether user input directed the viewport to move this frame.
    pub moving: bool,
    /// Whether user input directed the viewport to zoom in or out this frame.
    pub zooming: bool,
}
impl Index<u32> for InputState {
    type Output = bool;
    fn index(&self, scancode: u32) -> &bool {
        &self[&scancode]
    }
}
impl Index<VirtualKeyCode> for InputState {
    type Output = bool;
    fn index(&self, virtual_keycode: VirtualKeyCode) -> &bool {
        &self[&virtual_keycode]
    }
}
impl Index<&u32> for InputState {
    type Output = bool;
    fn index(&self, scancode: &u32) -> &bool {
        if self.scancodes.contains(scancode) {
            TRUE_REF
        } else {
            FALSE_REF
        }
    }
}
impl Index<&VirtualKeyCode> for InputState {
    type Output = bool;
    fn index(&self, virtual_keycode: &VirtualKeyCode) -> &bool {
        if self.virtual_keycodes.contains(virtual_keycode) {
            TRUE_REF
        } else {
            FALSE_REF
        }
    }
}
impl InputState {
    /// Update internal key state based on a KeyboardInput event.
    pub fn update(&mut self, input: &KeyboardInput) {
        match input.state {
            ElementState::Pressed => {
                self.scancodes.insert(input.scancode);
                if let Some(virtual_keycode) = input.virtual_keycode {
                    self.virtual_keycodes.insert(virtual_keycode);
                }
            }
            ElementState::Released => {
                self.scancodes.remove(&input.scancode);
                if let Some(virtual_keycode) = input.virtual_keycode {
                    self.virtual_keycodes.remove(&virtual_keycode);
                }
            }
        }
    }
}

pub fn start_frame(state: &mut super::State) {
    state.input_state.moving = false;
    state.input_state.zooming = false;
}

pub fn handle_event(state: &mut super::State, ev: &Event) {
    match ev {
        // Handle WindowEvents.
        Event::WindowEvent { event, .. } => {
            match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    state.input_state.update(input);
                    handle_key(state, input);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // Scroll 64x.
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                        MouseScrollDelta::PixelDelta(dpi::LogicalPosition { x, y }) => {
                            (*x as f32, *y as f32)
                        }
                    };
                    match &mut state.grid_view {
                        gridview::GridView::View2D(view2d) => {
                            view2d.viewport.scroll_pixels(dx * 64.0, dy * 64.0);
                        }
                        _ => (),
                    }
                }

                _ => (),
            }
        }
        // Ignore non-WindowEvents.
        _ => (),
    }
}

pub fn handle_key(state: &mut super::State, input: &KeyboardInput) {
    match input {
        // Handle key press.
        KeyboardInput {
            state: ElementState::Pressed,
            virtual_keycode,
            modifiers,
            ..
        } => {
            match modifiers {
                // No modifiers
                ModifiersState {
                    shift: false,
                    ctrl: false,
                    alt: false,
                    logo: false,
                } => match virtual_keycode {
                    // Handle spacebar press.
                    Some(VirtualKeyCode::Space) => {
                        // TODO single step with history
                    }
                    // Handle tab key press.
                    Some(VirtualKeyCode::Tab) => {
                        // TODO step with history
                    }
                    _ => (),
                },

                // CTRL
                ModifiersState {
                    shift: false,
                    ctrl: true,
                    alt: false,
                    logo: false,
                } =>
                    match virtual_keycode {
                        // Undo.
                        Some(VirtualKeyCode::Z) => {
                            state.history.undo(&mut state.grid_view);
                        }
                        // Redo.
                        Some(VirtualKeyCode::Y) => {
                            state.history.redo(&mut state.grid_view);
                        }
                        // Reset.
                        Some(VirtualKeyCode::R) => {
                            // TODO reset
                        }
                        _ => (),
                    }
                ,
                // SHIFT + CTRL
                ModifiersState {
                    shift: true,
                    ctrl: true,
                    alt: false,
                    logo: false,
                } =>
                    match virtual_keycode {
                        // Redo.
                        Some(VirtualKeyCode::Z) => {state.history.redo(&mut state.grid_view);},
                        _ => (),
                    }
                ,_=>(),
            }
        }
        // Ignore key release.
        _ => (),
    }
}

pub fn do_frame(state: &mut super::State) {
    match &mut state.grid_view {
        gridview::GridView::View2D(view2d) => {
            let input_state = &mut state.input_state;
            // 'A' or left arrow => scroll west.
            if input_state[30] || input_state[VirtualKeyCode::Left] {
                view2d.viewport.scroll_pixels(-32.0, 0.0);
                input_state.moving = true;
            }
            // 'D' or right arrow => scroll west.
            if input_state[32] || input_state[VirtualKeyCode::Right] {
                view2d.viewport.scroll_pixels(32.0, 0.0);
                input_state.moving = true;
            }
            // 'W' or up arrow => scroll north.
            if input_state[17] || input_state[VirtualKeyCode::Up] {
                view2d.viewport.scroll_pixels(0.0, 32.0);
                input_state.moving = true;
            }
            // 'S' or down arrow => scroll down.
            if input_state[31] || input_state[VirtualKeyCode::Down] {
                view2d.viewport.scroll_pixels(0.0, -32.0);
                input_state.moving = true;
            }
            // 'Q' or page up => zoom in.
            if input_state[16] || input_state[VirtualKeyCode::PageUp] {
                view2d.viewport.zoom_by(2.0f32.powf(0.1f32));
                input_state.zooming = true;
            }
            // 'Z' or page down => zoom out.
            if input_state[44] || input_state[VirtualKeyCode::PageDown] {
                view2d.viewport.zoom_by(2.0f32.powf(-0.1f32));
                input_state.zooming = true;
            }
            if !input_state.moving {
                // Snap to nearest position and zoom level.
                view2d.viewport.pos += Vec2D::from([
                    view2d.viewport.x_offset.round() as isize,
                    view2d.viewport.y_offset.round() as isize,
                ]);
                view2d.viewport.x_offset = 0.0;
                view2d.viewport.y_offset = 0.0;
            }
            if !input_state.zooming {
                view2d.viewport.zoom = view2d.viewport.zoom.round();
            }
        }
        gridview::GridView::View3D(_) => (),
    }
}
