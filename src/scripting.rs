use rhai::{Engine, EvalAltResult, Scope, RegisterFn, AST};
use crate::engine::{EMPTY, Particle, WorldView, Kind, World};
use rand::{thread_rng, Rng};
use rand::prelude::ThreadRng;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ScriptEngine {
    engine: Engine,
    script: AST,
}

impl ScriptEngine {
    pub(crate) fn new() -> Self {
        let mut engine = Engine::new();

        engine.register_type::<WorldView>();

        engine.register_fn("get", WorldView::get);
        engine.register_fn("set", WorldView::set);
        engine.register_fn("set_viewport", WorldView::set_viewport);

        engine.register_type::<Particle>();

        engine.register_get("kind", Particle::get_kind);
        engine.register_get("clock", Particle::get_clock);

        engine.register_type::<ThreadRng>();
        engine.register_fn("gen_bool", ThreadRng::gen_bool);

        let script = engine.compile(r"
            for x in range(0, width) {
                let x = if clock % 2 == 0 {
                    width - (1 + x)
                } else {
                    x
                };

                for y in range(0, height) {
                    view.set_viewport(x, y);
                    let current = view.get(0, 0);
                    if current.kind == KIND_EMPTY || current.clock == clock {
                        continue;
                    }

                    if current.kind == KIND_SAND {
                        let dx = if rng.gen_bool(0.5) { -1 } else { 1 };
                        let side = view.get(dx, 1);
                        let below = view.get(0, 1);
                        if below.kind == KIND_EMPTY {
                            view.set(0, 1, current);
                            view.set(0, 0, EMPTY);
                        } else if side.kind == KIND_EMPTY {
                            view.set(dx, 1, current);
                            view.set(0, 0, EMPTY);
                        } else {
                            view.set(0, 0, current);
                        }
                    }
                }
            }
        ").unwrap();

        Self {
            engine,
            script,
        }
    }

    pub(crate) fn tick(&mut self, clock: u8, width: i32, height: i32, view: WorldView) -> Result<(), Box<EvalAltResult>> {
        /*for x in 0..self.width {
            let x = if clock % 2 == 0 {
                self.width - (1 + x)
            } else {
                x
            };

            for y in 0..self.height {
                let current = self.world.borrow().get(x, y);
                if current.kind == Kind::Empty || current.clock == clock {
                    continue;
                }

                let mut view = WorldView {
                    x,
                    y,
                    world: self.world.clone(),
                };

                match current.kind {
                    Kind::Sand => {
                        self.script_engine.run(view, current.kind).unwrap();
                    }
                    Kind::Plant => {
                        /*
                        if rng.gen_bool(((current.extra.energy * 0.05) + 0.05) as f64) {
                            let cost = 0.02;
                            let mut growth_spots = [
                                (-1, -1), (1, -1), (-1, 0), (1, 0), (0, -1)];
                            growth_spots.shuffle(&mut rng);
                            let mut grown = false;

                            let mut nearby = 0;
                            for x in -2..=2 {
                                for y in -2..=2 {
                                    if view.get(x, y).kind == Kind::Plant {
                                        nearby += 1;
                                    }
                                }
                            }

                            for point in growth_spots.iter() {
                                let spot = view.get(point.0, point.1);
                                if spot.kind == Kind::Empty && nearby <= 20 && current.extra.energy > 0.0 && !grown {
                                    view.set(point.0, point.1, current.with_energy(current.extra.energy - cost).new_extra());
                                    grown = true;
                                } else if spot.kind == Kind::Water {
                                    view.set(point.0, point.1, current.with_energy(1.0).new_extra());
                                    grown = true;
                                }
                            }
                            if grown {
                                view.set(0, 0, current.with_energy(0.0));
                            } else {
                                view.set(0, 0, current.with_energy(current.extra.energy - cost / 2.0));
                            }
                        } else {
                            view.set(0, 0, current);
                        }
                    }
                    Kind::Fire => {
                        if current.extra.energy <= 0.0 {
                            view.set(0, 0, EMPTY);
                        } else {
                            let cost = 0.1;
                            view.set(0, 0, current.with_energy(current.extra.energy - cost));
                            let dx = rng.gen_range(-1, 2);
                            let dy = rng.gen_range(-1, 2);
                            if dx != 0 || dy != 0 {
                                let next = view.get(dx, dy);
                                if next.kind == Kind::Empty {
                                    view.set(dx, dy, current.with_energy(current.extra.energy - cost));
                                } else if next.kind == Kind::Plant {
                                    view.set(dx, dy, current.with_energy(1.0));
                                }
                            }
                        }
                    }
                    Kind::Water => {
                        let dx = if rng.gen_bool(0.5) { -1 } else { 1 };
                        let side = view.get(dx, 1);
                        let below = view.get(0, 1);
                        if below.kind == Kind::Empty || below.kind == Kind::Fire {
                            view.set(0, 1, current);
                            view.set(0, 0, EMPTY);
                        } else if side.kind == Kind::Empty || below.kind == Kind::Fire {
                            view.set(dx, 1, current);
                            view.set(0, 0, EMPTY);
                        } else {
                            view.set(0, 0, current);
                        }
                    }
                    _ => view.set(0, 0, current),
                }
                 */
            }
         */
        let mut scope = Scope::new();
        let rng = thread_rng();

        scope.push_constant("KIND_SAND", Kind::Sand.value());
        scope.push_constant("KIND_EMPTY", Kind::Empty.value());
        scope.push_constant("KIND_WATER", Kind::Water.value());
        scope.push_constant("KIND_FIRE", Kind::Fire.value());
        scope.push_constant("KIND_PLANT", Kind::Plant.value());
        scope.push_constant("EMPTY", EMPTY);
        scope.push("rng", rng);
        scope.push("clock", clock as i32);
        scope.push("width", width);
        scope.push("height", height);
        scope.push("view", view);

        self.engine.eval_ast_with_scope(&mut scope, &self.script)?;

        Ok(())
    }
}
