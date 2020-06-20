use pixel_canvas::{Canvas};
use pixel_canvas::input::{WindowEvent, Event};
use winit::event::{MouseButton, ElementState, VirtualKeyCode};
use std::thread;
use std::time::Duration;
use image::imageops::{resize, FilterType};
use rand::{thread_rng, Rng};
use crossbeam::channel::{Sender, Receiver, unbounded};
use rand::seq::SliceRandom;
use colors_transform::{Rgb, Color};
use pixel_canvas::canvas::CanvasInfo;

#[derive(Clone, Eq, PartialEq, Debug, Copy)]
pub enum Kind {
    Sand,
    Plant,
    Empty,
    OutOfBounds,
}

#[derive(Clone, PartialEq, Debug, Copy)]
struct Particle {
    x: i32,
    y: i32,
    kind: Kind,
    extra: Extra,
    clock: u8,
}

impl Particle {
    fn _not(&self, kind: Kind) -> bool {
        self.kind != kind
    }

    fn is(&self, kind: Kind) -> bool {
        self.kind == kind
    }

    fn _same_as(&self, other: Particle) -> bool {
        self.is(other.kind)
    }

    fn with_position_of(&self, other: Particle) -> Particle {
        let mut new = self.clone();
        new.x = other.x;
        new.y = other.y;
        new
    }

    fn with_energy(&self, energy: f64) -> Particle {
        let mut new = self.clone();
        new.extra.energy = energy;
        if new.extra.energy < 0.0 {
            new.extra.energy = 0.0;
        } else if new.extra.energy > 1.0 {
            new.extra.energy = 1.0;
        }
        new
    }

    fn reduce_energy(&mut self, amount: f64) {
        self.extra.energy -= amount;
        if self.extra.energy < 0.0 {
            self.extra.energy = 0.0;
        } else if self.extra.energy > 1.0 {
            self.extra.energy = 1.0;
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
struct Extra {
    color: ParticleColor,
    energy: f64,
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
struct ParticleColor {
    r: u8,
    g: u8,
    b: u8,
}

impl ParticleColor {
    fn to_canvas(&self) -> pixel_canvas::Color {
        pixel_canvas::Color { r: self.r, g: self.g, b: self.b }
    }
}

impl Extra {
    fn new() -> Extra {
        Default::default()
    }

    fn from(kind: Kind) -> Extra {
        let mut rng = thread_rng();
        match kind {
            Kind::Sand => {
                let rgb = Rgb::from(237.0, 201.0, 175.0);
                let rgb = rgb.lighten(rng.gen_range(-4.0, 4.0));
                Self {
                    color: ParticleColor {
                        r: rgb.get_red() as u8,
                        g: rgb.get_green() as u8,
                        b: rgb.get_blue() as u8,
                    },
                    energy: 0.0,
                }
            }
            Kind::Plant => {
                Self {
                    color: ParticleColor {
                        r: 0,
                        g: 200,
                        b: 0,
                    },
                    energy: 1.0,
                }
            }
            Kind::Empty => {
                Self {
                    color: ParticleColor { r: 0, g: 0, b: 0 },
                    energy: 0.0,
                }
            }
            Kind::OutOfBounds => {
                Self {
                    color: ParticleColor { r: 0, g: 0, b: 0 },
                    energy: 0.0,
                }
            }
        }
    }
}

struct Update {
    particle: Particle,
}

struct UserEvent {
    x: i32,
    y: i32,
    kind: Kind,
}

#[derive(Clone)]
struct Sandbox {
    width: i32,
    height: i32,
    world: Vec<Vec<Particle>>,
    update_tx: Sender<Update>,
    event_rx: Receiver<UserEvent>,
    clock: u8,
}

impl Sandbox {
    fn new(width: i32, height: i32, update_tx: Sender<Update>, event_rx: Receiver<UserEvent>) -> Sandbox {
        return Sandbox {
            width,
            height,
            world: vec![vec![Particle { x: 0, y: 0, kind: Kind::Empty, clock: 0, extra: Extra::new() };
                             width as usize];
                        height as usize],
            update_tx,
            event_rx,
            clock: 0,
        };
    }

    fn init(&mut self) {
        let image = image::open("picture.png").unwrap();
        let image = resize(
            &image, self.width as u32, self.height as u32, FilterType::Lanczos3);

        for (x, y, pixel) in image.enumerate_pixels() {
            let x = x as i32;
            let y = y as i32;
            let [r, g, b, _] = pixel.0;
            let rgb = Rgb::from(r as f32, g as f32, b as f32);
            let particle = if false && rgb.get_lightness() > 40.0 {
                let rgb = rgb.set_hue(52.0);
                Particle {
                    x,
                    y,
                    kind: Kind::Sand,
                    extra: Extra {
                        color: ParticleColor {
                            r: rgb.get_red() as u8,
                            g: rgb.get_green() as u8,
                            b: rgb.get_blue() as u8,
                        },
                        energy: 0.0,
                    },
                    clock: 0,
                }
            } else {
                Particle {
                    x,
                    y,
                    kind: Kind::Empty,
                    extra: Extra {
                        color: ParticleColor { r: 0, g: 0, b: 0 },
                        energy: 0.0,
                    },
                    clock: 0,
                }
            };
            self.world[y as usize][x as usize] = particle;
            self.update_tx.send(Update { particle }).unwrap();
        }
    }

    fn get(&self, x: i32, y: i32) -> Particle {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            Particle { x, y, kind: Kind::OutOfBounds, extra: Extra::new(), clock: 0 }
        } else {
            self.world[y as usize][x as usize]
        }
    }

    fn set(&mut self, x: i32, y: i32, mut particle: Particle) {
        particle.x = x;
        particle.y = y;
        self.world[y as usize][x as usize] = particle;
        self.update_tx.send(Update { particle }).unwrap();
    }

    fn insert(&mut self, particle: Particle) {
        self.world[particle.y as usize][particle.x as usize] = particle;
        self.update_tx.send(Update { particle }).unwrap();
    }

    fn swap(&mut self, p1: Particle, p2: Particle) {
        let p2_x = p2.x;
        let p2_y = p2.y;
        self.set(p1.x, p1.y, p2);
        self.set(p2_x, p2_y, p1);
    }

    fn tick(&mut self) {
        let (clock, _) = self.clock.overflowing_add(1);
        self.clock = clock;

        let mut rng = thread_rng();

        let mut xs: Vec<i32> = (0..self.width).collect();
        let mut ys: Vec<i32> = (0..self.height).collect();

        xs.shuffle(&mut rng);
        ys.shuffle(&mut rng);

        for y in ys.iter() {
            for x in xs.iter() {
                let x = *x;
                let y = *y;
                let mut current = self.get(x, y);

                if current.clock == self.clock {
                    continue;
                }
                current.clock = self.clock;

                let relative = |rel_x: i32, rel_y: i32| -> Particle {
                    let x = x + rel_x;
                    let y = y + rel_y;
                    self.get(x, y)
                };

                let below = relative(0, 1);
                let top_left = relative(-1, -1);
                let top_right = relative(1, -1);
                let bottom_left = relative(-1, 1);
                let bottom_right = relative(1, 1);
                let left = relative(-1, 0);
                let right = relative(1, 0);
                let above = relative(0, -1);

                if current.is(Kind::Sand) {
                    if below.is(Kind::Empty) {
                        self.swap(current, below);
                    } else if bottom_left.is(Kind::Empty) && bottom_right.is(Kind::Empty) {
                        if rng.gen_bool(0.5) {
                            self.swap(current, bottom_left);
                        } else {
                            self.swap(current, bottom_right);
                        }
                    } else if bottom_left.is(Kind::Empty) {
                        self.swap(current, bottom_left);
                    } else if bottom_right.is(Kind::Empty) {
                        self.swap(current, bottom_right);
                    }
                } else if current.is(Kind::Plant) {
                    if current.extra.energy > 0.0 {
                        let mut nearby = 0;
                        for x in -2..=2 {
                            for y in -2..=2 {
                                if relative(x, y).is(Kind::Plant) {
                                    nearby += 1;
                                }
                            }
                        }

                        if nearby > 12 {
                            current.extra.energy = 0.0;
                        } else if rng.gen_bool(current.extra.energy * 0.05) {
                            let cost = 0.02;
                            let mut growth_spots = [top_left, top_right, left, right, above];
                            growth_spots.shuffle(&mut rng);
                            for spot in growth_spots.iter() {
                                if spot.is(Kind::Empty) {
                                    current.reduce_energy(cost / 2.0);
                                    self.insert(current
                                        .with_position_of(*spot)
                                        .with_energy(current.extra.energy - cost));
                                    current.extra.energy = 0.0;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        loop {
            match self.event_rx.try_recv() {
                Ok(event) => {
                    for x in -3..=3 {
                        for y in -3..=3 {
                            let x = x + event.x;
                            let y = y + event.y;
                            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                                continue;
                            }

                            self.set(x, y, Particle {
                                x,
                                y,
                                kind: event.kind,
                                extra: Extra::from(event.kind),
                                clock: self.clock,
                            });
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }
}

fn main() {
    let width = 500;
    let height = 500;

    let canvas = Canvas::new(width, height)
        .title("Sandbox")
        .state(GuiState::new())
        .input(GuiState::handle_input);

    let (update_tx, update_rx) = unbounded();
    let (event_tx, event_rx) = unbounded();
    let mut sandbox = Sandbox::new(width as i32, height as i32, update_tx, event_rx);
    sandbox.init();

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        loop {
            sandbox.tick();
            thread::sleep(Duration::from_millis(5));
        }
    });

    canvas.render(move |gui_state, image| {
        let width = image.width() as usize;
        loop {
            match update_rx.try_recv() {
                Ok(update) => {
                    let new_color = update.particle.extra.color.to_canvas();
                    image.chunks_mut(width)
                         .nth(height - update.particle.y as usize - 1)
                         .unwrap()[update.particle.x as usize] = new_color;
                }
                Err(_) => break,
            }
        }

        if gui_state.down &&
            gui_state.x >= 0 && gui_state.x < width as i32 &&
            gui_state.y >= 0 && gui_state.y < height as i32 {
            event_tx.send(UserEvent {
                x: gui_state.x,
                y: gui_state.y,
                kind: gui_state.kind,
            }).unwrap();
        }
    });
}

pub struct GuiState {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
    pub down: bool,
}

impl GuiState {
    /// Create a MouseState. For use with the `state` method.
    pub fn new() -> Self {
        Self {
            kind: Kind::Sand,
            x: 0,
            y: 0,
            down: false,
        }
    }

    /// Handle input for the mouse. For use with the `input` method.
    pub fn handle_input(_: &CanvasInfo, gui_state: &mut GuiState, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent {
                event, ..
            } => {
                match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        match input.state {
                            ElementState::Pressed => {
                                match input.virtual_keycode.unwrap() {
                                    VirtualKeyCode::Key1 => gui_state.kind = Kind::Sand,
                                    VirtualKeyCode::Key2 => gui_state.kind = Kind::Plant,
                                    VirtualKeyCode::Key0 => gui_state.kind = Kind::Empty,
                                    _ => {}
                                }
                            }
                            ElementState::Released => {}
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let (x, y): (i32, i32) = (*position).into();

                        gui_state.x = (x as f32 * 0.5) as i32;
                        gui_state.y = (y as f32 * 0.5) as i32;
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        match button {
                            MouseButton::Left => {
                                match state {
                                    ElementState::Pressed => gui_state.down = true,
                                    ElementState::Released => gui_state.down = false,
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }
}
