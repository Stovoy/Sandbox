use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, Clamped};
use std::cell::{RefCell, Cell};
use std::rc::Rc;
use crate::engine::{Kind, UserEvent, Sandbox};

pub mod engine;

struct Renderer {
    canvas: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
}

impl Renderer {
    fn new() -> Self {
        let canvas = document().get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        // context.set_image_smoothing_enabled(false);

        Self {
            canvas,
            context,
        }
    }

    fn draw_canvas(&self, canvas: &web_sys::HtmlCanvasElement) {
        self.context.draw_image_with_html_canvas_element(
            canvas, 0.0, 0.0).unwrap();
    }
}

#[wasm_bindgen]
pub struct IntervalHandle {
    interval_id: i32,
    _closure: Closure<dyn FnMut()>,
}

impl Drop for IntervalHandle {
    fn drop(&mut self) {
        let window = web_sys::window().unwrap();
        window.clear_interval_with_handle(self.interval_id);
    }
}

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

#[wasm_bindgen]
pub fn run() -> Result<IntervalHandle, JsValue> {
    #[cfg(debug_assertions)]
        console_error_panic_hook::set_once();

    let renderer = Renderer::new();
    let canvas = renderer.canvas.clone();

    let width: usize = 400;
    let height: usize = 400;

    canvas.set_width(width as u32);
    canvas.set_height(width as u32);

    let mut sandbox = Sandbox::new(width as i32, height as i32);
    let world = sandbox.world();

    let gui_state = Rc::new(Cell::new(GuiState::new()));

    let gui_state_tick = gui_state.clone();
    let tick = Closure::wrap(Box::new(move || {
        let gui_state = gui_state_tick.get();
        let user_event = if gui_state.down &&
            gui_state.x >= 0 && gui_state.x < width as i32 &&
            gui_state.y >= 0 && gui_state.y < height as i32 {
            Some(UserEvent {
                x: gui_state.x,
                y: gui_state.y,
                kind: gui_state.kind,
                size: gui_state.size,
            })
        } else {
            None
        };

        sandbox.tick(user_event);
    }) as Box<dyn FnMut()>);

    let render = Rc::new(RefCell::new(None));
    let render_clone = render.clone();

    let canvas_buffer = document().create_element("canvas")
                                  .unwrap()
                                  .dyn_into::<web_sys::HtmlCanvasElement>()
                                  .unwrap();
    canvas_buffer.set_width(width as u32);
    canvas_buffer.set_height(height as u32);
    let context_buffer = canvas_buffer
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    *render_clone.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let mut image_index = 0;
        let mut data: Vec<u8> = vec![255; width * height * 4];
        for index in 0..(width * height) {
            let particle = unsafe { &*world.add(index) };
            let color = particle.extra.color;

            data[image_index] = color.r;
            data[image_index + 1] = color.g;
            data[image_index + 2] = color.b;
            image_index += 4;
        }

        let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut data), width as u32, height as u32).unwrap();

        context_buffer.put_image_data(&data, 0.0, 0.0).unwrap();
        renderer.draw_canvas(&canvas_buffer);

        request_animation_frame(render.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(render_clone.borrow().as_ref().unwrap());

    let interval_id = window()
        .set_interval_with_callback_and_timeout_and_arguments_0(
            tick.as_ref().unchecked_ref(), 0)?;

    {
        let gui_state = gui_state.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let mut gui_state_inner = gui_state.get();
            gui_state_inner.down = true;
            gui_state.set(gui_state_inner);
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let gui_state = gui_state.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            let mut gui_state_inner = gui_state.get();
            gui_state_inner.down = false;
            gui_state.set(gui_state_inner);
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let gui_state = gui_state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut gui_state_inner = gui_state.get();
            gui_state_inner.x = event.offset_x();
            gui_state_inner.y = event.offset_y();
            gui_state.set(gui_state_inner);
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let gui_state = gui_state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut gui_state_inner = gui_state.get();
            match event.key().as_str() {
                "1" => gui_state_inner.kind = Kind::Sand,
                "2" => gui_state_inner.kind = Kind::Plant,
                "3" => gui_state_inner.kind = Kind::Fire,
                "e" => gui_state_inner.kind = Kind::Empty,
                "+" => if gui_state_inner.size < width as u32 { gui_state_inner.size += 1 },
                "-" => if gui_state_inner.size > 1 { gui_state_inner.size -= 1 },
                _ => {}
            }
            gui_state.set(gui_state_inner);
        }) as Box<dyn FnMut(_)>);

        document().add_event_listener_with_callback("keypress", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(IntervalHandle {
        interval_id,
        _closure: tick,
    })
}

#[derive(Copy, Clone)]
pub(crate) struct GuiState {
    pub(crate) kind: Kind,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) down: bool,
    pub(crate) size: u32,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            kind: Kind::Sand,
            x: 0,
            y: 0,
            size: 25,
            down: false,
        }
    }
}
