use pixel_canvas::{Canvas, input::MouseState};
use std::thread;
use std::time::Duration;
use image::imageops::{resize, FilterType};
use rand::{thread_rng, Rng};
use crossbeam::channel::{Sender, unbounded};
use rand::seq::SliceRandom;
use colors_transform::{Rgb, Color};

#[derive(Clone, Eq, PartialEq, Debug, Copy)]
enum Kind {
    Sand,
    Empty,
    OutOfBounds,
}

#[derive(Clone, Eq, PartialEq, Debug, Copy)]
struct Particle {
    x: i32,
    y: i32,
    kind: Kind,
    extra: Extra,
    clock: u8,
}

impl Particle {
    fn not(&self, kind: Kind) -> bool {
        self.kind != kind
    }

    fn is(&self, kind: Kind) -> bool {
        self.kind == kind
    }

    fn _same_as(&self, other: Particle) -> bool {
        self.is(other.kind)
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
struct Extra {
    color: ParticleColor,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
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
        Extra {
            color: ParticleColor {
                r: 0,
                g: 0,
                b: 0,
            }
        }
    }
}

struct Update {
    particle: Particle,
}

#[derive(Clone)]
struct Sandbox {
    width: i32,
    height: i32,
    world: Vec<Vec<Particle>>,
    update_tx: Sender<Update>,
    clock: u8,
}

impl Sandbox {
    fn new(width: i32, height: i32, update_tx: Sender<Update>) -> Sandbox {
        return Sandbox {
            width,
            height,
            world: vec![vec![Particle { x: 0, y: 0, kind: Kind::Empty, clock: 0, extra: Extra::new() };
                             width as usize];
                        height as usize],
            update_tx,
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
            let particle = if rgb.get_lightness() > 40.0 {
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
                        }
                    },
                    clock: 0,
                }
            } else {
                Particle {
                    x,
                    y,
                    kind: Kind::Empty,
                    extra: Extra {
                        color: ParticleColor { r: 0, g: 0, b: 0 }
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

    fn update(&mut self, x: i32, y: i32, mut particle: Particle) {
        particle.x = x;
        particle.y = y;
        self.world[y as usize][x as usize] = particle;
        self.update_tx.send(Update { particle }).unwrap();
    }

    fn swap(&mut self, p1: Particle, p2: Particle) {
        let p2_x = p2.x;
        let p2_y = p2.y;
        self.update(p1.x, p1.y, p2);
        self.update(p2_x, p2_y, p1);
    }

    fn tick(&mut self) {
        self.clock += 1;

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
                let bottom_left = relative(-1, 1);
                let bottom_right = relative(1, 1);

                if current.not(Kind::Empty) {
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
                }
            }
        }
    }
}

fn main() {
    let width = 500;
    let height = 500;

    let canvas = Canvas::new(width, height)
        .title("Sandbox")
        .state(MouseState::new())
        .input(MouseState::handle_input);

    let (tx, rx) = unbounded();
    let mut sandbox = Sandbox::new(width as i32, height as i32, tx);
    sandbox.init();

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        loop {
            sandbox.tick();
        }
    });

    canvas.render(move |_mouse, image| {
        let width = image.width() as usize;
        loop {
            match rx.try_recv() {
                Ok(update) => {
                    let new_color = match update.particle.kind {
                        Kind::Sand => Some(update.particle.extra.color.to_canvas()),
                        Kind::Empty => Some(pixel_canvas::Color { r: 0, g: 0, b: 0 }),
                        Kind::OutOfBounds => None,
                    };

                    if let Some(new_color) = new_color {
                        image.chunks_mut(width)
                             .nth(height - update.particle.y as usize - 1)
                             .unwrap()[update.particle.x as usize] = new_color;
                    }
                }
                Err(_) => break,
            }
        }
    });
}
