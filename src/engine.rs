use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;
use colors_transform::{Rgb, Color as ColorTransform};

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

static EMPTY: Particle = Particle {
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

#[derive(Clone, Eq, PartialEq, Debug, Copy)]
pub enum Kind {
    Sand,
    Plant,
    Empty,
    OutOfBounds,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub struct Particle {
    kind: Kind,
    pub extra: Extra,
    clock: u8,
}

impl Particle {
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
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct Extra {
    pub color: Color,
    energy: f64,
}

#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Extra {
    fn from(kind: Kind) -> Extra {
        let mut rng = thread_rng();
        match kind {
            Kind::Sand => {
                let rgb = Rgb::from(237.0, 201.0, 175.0);
                let rgb = rgb.lighten(rng.gen_range(-4.0, 4.0));
                Self {
                    color: Color {
                        r: rgb.get_red() as u8,
                        g: rgb.get_green() as u8,
                        b: rgb.get_blue() as u8,
                    },
                    energy: 0.0,
                }
            }
            Kind::Plant => {
                Self {
                    color: Color {
                        r: 0,
                        g: 200,
                        b: 0,
                    },
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
}

pub struct UserEvent {
    pub x: i32,
    pub y: i32,
    pub kind: Kind,
    pub size: u32,
}

#[derive(Clone)]
pub struct Sandbox {
    width: i32,
    height: i32,
    world: Vec<Particle>,
    clock: u8,
}

struct SandboxView<'a> {
    x: i32,
    y: i32,
    sandbox: &'a mut Sandbox,
}

impl<'a> SandboxView<'a> {
    fn get(&self, d_x: i32, d_y: i32) -> Particle {
        let x = self.x + d_x;
        let y = self.y + d_y;

        self.sandbox.get(x, y)
    }

    fn set(&mut self, d_x: i32, d_y: i32, particle: Particle) {
        let x = self.x + d_x;
        let y = self.y + d_y;

        self.sandbox.set(x, y, particle);
    }
}

#[derive(Clone)]
pub struct ParticlePoint {
    pub x: i32,
    pub y: i32,
    pub particle: Particle,
}

impl Sandbox {
    pub fn new(width: i32, height: i32) -> Self {
        let world = vec![OUT_OF_BOUNDS; (width * height) as usize];
        let mut sandbox = Self {
            width,
            height,
            world,
            clock: 0,
        };

        for x in 0..width {
            for y in 0..height {
                sandbox.set(
                    x, y,
                    Particle {
                        kind: Kind::Empty,
                        extra: Default::default(),
                        clock: 0,
                    },
                )
            }
        }

        sandbox
    }

    fn get_index(&self, x: i32, y: i32) -> usize {
        return (x + y * self.height) as usize;
    }

    fn is_out_of_bounds(&self, x: i32, y: i32) -> bool {
        x < 0 || x >= self.width || y < 0 || y >= self.height
    }

    pub fn world(&self) -> *const Particle {
        self.world.as_ptr()
    }

    fn get(&self, x: i32, y: i32) -> Particle {
        if self.is_out_of_bounds(x, y) {
            OUT_OF_BOUNDS
        } else {
            self.world[self.get_index(x, y)].clone()
        }
    }

    fn set(&mut self, x: i32, y: i32, particle: Particle) {
        let index = self.get_index(x, y);
        self.world[index] = particle;
        self.world[index].clock = self.clock;
    }

    pub fn tick(&mut self, user_event: Option<UserEvent>) {
        let (clock, _) = self.clock.overflowing_add(1);
        self.clock = clock;

        let mut rng = thread_rng();

        for x in 0..self.width {
            let x = if self.clock % 2 == 0 {
                self.width - (1 + x)
            } else {
                x
            };

            for y in 0..self.height {
                let current = self.get(x, y);
                if current.kind == Kind::Empty || current.clock == clock {
                    continue;
                }

                let mut view = SandboxView {
                    x,
                    y,
                    sandbox: self,
                };

                match current.kind {
                    Kind::Sand => {
                        let dx = if rng.gen_bool(0.5) { -1 } else { 1 };
                        let side = view.get(dx, 1);
                        let below = view.get(0, 1);
                        if below.kind == Kind::Empty {
                            view.set(0, 1, current);
                            view.set(0, 0, EMPTY);
                        } else if side.kind == Kind::Empty {
                            view.set(dx, 1, current);
                            view.set(0, 0, EMPTY);
                        } else {
                            view.set(0, 0, current);
                        }
                    }
                    Kind::Plant => {
                        if current.extra.energy > 0.0 {
                            let mut nearby = 0;
                            for x in -2..=2 {
                                for y in -2..=2 {
                                    if view.get(x, y).kind == Kind::Plant {
                                        nearby += 1;
                                    }
                                }
                            }

                            if nearby > 20 {
                                view.set(0, 0, current.with_energy(0.0));
                            } else if rng.gen_bool(current.extra.energy * 0.05) {
                                let cost = 0.02;
                                let mut growth_spots = [
                                    (-1, -1), (1, -1), (-1, 0), (1, 0), (0, -1)];
                                growth_spots.shuffle(&mut rng);
                                let mut grown = false;
                                for point in growth_spots.iter() {
                                    let spot = view.get(point.0, point.1);
                                    if spot.kind == Kind::Empty {
                                        view.set(point.0, point.1, current.with_energy(current.extra.energy - cost));
                                        view.set(0, 0, current.with_energy(0.0));
                                        grown = true;
                                        break;
                                    }
                                }
                                if !grown {
                                    view.set(0, 0, current.with_energy(current.extra.energy - cost / 2.0));
                                }
                            }
                        } else {
                            view.set(0, 0, current);
                        }
                    }
                    _ => view.set(0, 0, current),
                }
            }
        }

        match user_event {
                Some(event) => {
                    let size = event.size as i32;
                    for x in -size..=size {
                        for y in -size..=size {
                            let x = x + event.x;
                            let y = y + event.y;
                            if self.is_out_of_bounds(x, y) {
                                continue;
                            }

                            self.set(x, y, Particle {
                                kind: event.kind,
                                extra: Extra::from(event.kind),
                                clock: self.clock,
                            });
                        }
                    }
                }
                None => {}
        }
    }
}
