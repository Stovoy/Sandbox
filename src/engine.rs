use rand::{thread_rng, Rng};
use colors_transform::{Rgb, Color as ColorTransform};
use std::rc::Rc;
use std::cell::RefCell;
use crate::scripting::ScriptEngine;

static OUT_OF_BOUNDS: Particle = Particle {
    kind: Kind::OutOfBounds,
    extra: Extra {
        color: Color {
            r: 0,
            g: 0,
            b: 0,
        },
        energy: 0.0,
    },
    clock: 0,
};

pub(crate) static EMPTY: Particle = Particle {
    kind: Kind::Empty,
    extra: Extra {
        color: Color {
            r: 0,
            g: 0,
            b: 0,
        },
        energy: 0.0,
    },
    clock: 0,
};

#[derive(Clone, Eq, PartialEq, Debug, Copy, Hash)]
pub enum Kind {
    Sand = 0,
    Plant = 1,
    Fire = 2,
    Water = 3,
    Empty = 4,
    OutOfBounds = 5,
}

impl Kind {
    pub(crate) fn value(&self) -> i32 {
        *self as i32
    }
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub struct Particle {
    pub kind: Kind,
    pub extra: Extra,
    clock: u8,
}

impl Particle {
    pub(crate) fn get_kind(&mut self) -> i32 {
        self.kind.value()
    }

    pub(crate) fn get_clock(&mut self) -> i32 {
        self.clock as i32
    }

    fn with_energy(&self, energy: f32) -> Particle {
        let mut new = self.clone();
        new.extra.energy = energy;
        if new.extra.energy < 0.0 {
            new.extra.energy = 0.0;
        } else if new.extra.energy > 1.0 {
            new.extra.energy = 1.0;
        }
        new.extra.update(self.kind);
        new
    }

    fn new_extra(&self) -> Particle {
        let mut new = self.clone();
        new.extra = Extra::from(self.kind);
        new
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    fn from_rgb(rgb: Rgb) -> Self {
        Self {
            r: rgb.get_red() as u8,
            g: rgb.get_green() as u8,
            b: rgb.get_blue() as u8,
        }
    }

    fn to_rgb(&self) -> Rgb {
        Rgb::from(self.r as f32, self.g as f32, self.b as f32)
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct Extra {
    pub color: Color,
    energy: f32,
}

impl Extra {
    fn from(kind: Kind) -> Extra {
        let mut rng = thread_rng();
        match kind {
            Kind::Sand => {
                let rgb = Rgb::from(237.0, 201.0, 175.0);
                let rgb = rgb.lighten(rng.gen_range(-4.0, 4.0));
                Self {
                    color: Color::from_rgb(rgb),
                    energy: 0.0,
                }
            }
            Kind::Plant => {
                let rgb = Rgb::from(0.0, 200.0, 0.0);
                let rgb = rgb.lighten(rng.gen_range(-4.0, 4.0));
                Self {
                    color: Color::from_rgb(rgb),
                    energy: 1.0,
                }
            }
            Kind::Fire => {
                let rgb = Rgb::from(200.0, 0.0, 0.0);
                Self {
                    color: Color::from_rgb(rgb),
                    energy: 1.0,
                }
            }
            Kind::Water => {
                let rgb = Rgb::from(0.0, 0.0, 200.0);
                Self {
                    color: Color::from_rgb(rgb),
                    energy: 1.0,
                }
            }
            Kind::Empty | Kind::OutOfBounds => {
                Self {
                    color: Color { r: 0, g: 0, b: 0 },
                    energy: 0.0,
                }
            }
        }
    }

    fn update(&mut self, kind: Kind) {
        match kind {
            Kind::Fire => {
                let rgb = self.color.to_rgb().set_lightness(self.energy * 80.0);
                self.color = Color::from_rgb(rgb);
            }
            _ => {}
        }
    }
}

pub struct UserEvent {
    pub x: i32,
    pub y: i32,
    pub kind: Kind,
    pub size: u32,
}

pub struct World {
    width: i32,
    height: i32,
    data: Vec<Particle>,
    clock: u8,
}

impl World {
    fn new(width: i32, height: i32) -> Self {
        let data = vec![OUT_OF_BOUNDS; (width * height) as usize];
        Self {
            width,
            height,
            data,
            clock: 0,
        }
    }

    fn is_out_of_bounds(&self, x: i32, y: i32) -> bool {
        x < 0 || x >= self.width || y < 0 || y >= self.height
    }

    fn get_index(&self, x: i32, y: i32) -> usize {
        return (x + y * self.height) as usize;
    }

    fn get(&self, x: i32, y: i32) -> Particle {
        if self.is_out_of_bounds(x, y) {
            OUT_OF_BOUNDS
        } else {
            self.data[self.get_index(x, y)]
        }
    }

    fn set(&mut self, x: i32, y: i32, particle: Particle) {
        if self.is_out_of_bounds(x, y) {
            return;
        }
        let index = self.get_index(x, y);
        self.data[index] = particle;
        self.data[index].clock = self.clock;
    }
}

#[derive(Clone)]
pub(crate) struct WorldView {
    x: i32,
    y: i32,
    world: Rc<RefCell<World>>,
}

impl WorldView {
    pub(crate) fn get(&mut self, d_x: i32, d_y: i32) -> Particle {
        let x = self.x + d_x;
        let y = self.y + d_y;

        self.world.borrow().get(x, y)
    }

    pub(crate) fn set(&mut self, d_x: i32, d_y: i32, particle: Particle) {
        let x = self.x + d_x;
        let y = self.y + d_y;

        self.world.borrow_mut().set(x, y, particle);
    }

    pub(crate) fn set_viewport(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}

pub struct Sandbox {
    width: i32,
    height: i32,
    world: Rc<RefCell<World>>,
    script_engine: ScriptEngine,
}

#[derive(Clone)]
pub struct ParticlePoint {
    pub x: i32,
    pub y: i32,
    pub particle: Particle,
}

impl Sandbox {
    pub fn new(width: i32, height: i32) -> Self {
        let mut world = World::new(width, height);

        for x in 0..width {
            for y in 0..height {
                world.set(
                    x, y,
                    Particle {
                        kind: Kind::Empty,
                        extra: Default::default(),
                        clock: 0,
                    },
                )
            }
        }

        let world = Rc::new(RefCell::new(world));

        let sandbox = Self {
            width,
            height,
            world,
            script_engine: ScriptEngine::new(),
        };

        sandbox
    }

    pub fn world(&self) -> *const Particle {
        self.world.borrow().data.as_ptr()
    }

    pub fn tick(&mut self, user_event: Option<UserEvent>) {
        let (clock, _) = self.world.borrow().clock.overflowing_add(1);
        self.world.borrow_mut().clock = clock;

        let mut view = WorldView {
            x: 0, y: 0,
            world: self.world.clone(),
        };

        self.script_engine.tick(clock, self.width, self.height, view).unwrap();

        match user_event {
            Some(event) => {
                let size = event.size as i32;
                for x in -size..=size {
                    for y in -size..=size {
                        let x = x + event.x;
                        let y = y + event.y;

                        self.world.borrow_mut().set(x, y, Particle {
                            kind: event.kind,
                            extra: Extra::from(event.kind),
                            clock,
                        });
                    }
                }
            }
            None => {}
        }
    }
}
