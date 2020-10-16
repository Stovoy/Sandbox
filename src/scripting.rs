use rhai::{Engine, EvalAltResult, Scope, RegisterFn, AST};
use crate::engine::{EMPTY, Particle, WorldView, Kind};
use rand::{thread_rng, Rng};
use rand::prelude::ThreadRng;
use wasm_bindgen::prelude::*;
use walrus::ir::*;
use walrus::{FunctionBuilder, Module, ModuleConfig, ValType};
use wasm_bindgen::JsCast;

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

        // Construct a new Walrus module.
        let config = ModuleConfig::new();
        let mut module = Module::with_config(config);

        // Building this factorial implementation:
        // https://github.com/WebAssembly/testsuite/blob/7816043/fac.wast#L46-L66
        let mut factorial = FunctionBuilder::new(&mut module.types, &[ValType::I32], &[ValType::I32]);

        // Create our paramter and our two locals.
        let n = module.locals.add(ValType::I32);
        let i = module.locals.add(ValType::I32);
        let res = module.locals.add(ValType::I32);

        factorial
            // Enter the function's body.
            .func_body()
            // (local.set $i (local.get $n))
            .local_get(n)
            .local_set(i)
            // (local.set $res (i32.const 1))
            .i32_const(100)
            .local_set(res)
            .block(None, |done| {
                let done_id = done.id();
                done.loop_(None, |loop_| {
                    let loop_id = loop_.id();
                    loop_
                        // (i32.eq (local.get $i) (i32.const 0))
                        .local_get(i)
                        .i32_const(0)
                        .binop(BinaryOp::I32Eq)
                        .if_else(
                            None,
                            |then| {
                                // (then (br $done))
                                then.br(done_id);
                            },
                            |else_| {
                                else_
                                    // (local.set $res (i32.mul (local.get $i) (local.get $res)))
                                    .i32_const(100)
                                    .local_get(res)
                                    .binop(BinaryOp::I32Mul)
                                    .local_set(res)
                                    // (local.set $i (i32.sub (local.get $i) (i32.const 1))))
                                    .local_get(i)
                                    .i32_const(1)
                                    .binop(BinaryOp::I32Sub)
                                    .local_set(i);
                            },
                        )
                        .br(loop_id);
                });
            })
            .local_get(res);

        let factorial = factorial.finish(vec![n], &mut module.funcs);

        // Export the `factorial` function.
        module.exports.add("factorial", factorial);

        // Faster way than doing reflection
        // https://github.com/rustwasm/wasm-bindgen/issues/1428
        let wasm_bytes = module.emit_wasm();
        let descriptor = &js_sys::Object::new();
        js_sys::Reflect::set(&descriptor, &"initial".into(), &JsValue::from(256)).unwrap();
        js_sys::Reflect::set(&descriptor, &"maximum".into(), &JsValue::from(256)).unwrap();
        let memory = js_sys::WebAssembly::Memory::new(&descriptor).unwrap();
        let import_object = &js_sys::Object::new();
        let env_object = &js_sys::Object::new();
        let table_descriptor = &js_sys::Object::new();
        js_sys::Reflect::set(&table_descriptor, &"initial".into(), &JsValue::from(0)).unwrap();
        js_sys::Reflect::set(&table_descriptor, &"maximum".into(), &JsValue::from(0)).unwrap();
        js_sys::Reflect::set(&table_descriptor, &"element".into(), &JsValue::from("anyfunc")).unwrap();
        js_sys::Reflect::set(&env_object, &"table".into(),
                             &js_sys::WebAssembly::Table::new(&table_descriptor).unwrap()).unwrap();
        js_sys::Reflect::set(&env_object, &"tableBase".into(), &JsValue::from(0)).unwrap();
        js_sys::Reflect::set(&env_object, &"memory".into(), &memory).unwrap();
        js_sys::Reflect::set(&env_object, &"memoryBase".into(), &JsValue::from(1024)).unwrap();
        js_sys::Reflect::set(&env_object, &"STACKTOP".into(), &JsValue::from(0)).unwrap();
        js_sys::Reflect::set(&env_object, &"STACK_MAX".into(), &JsValue::from(256)).unwrap();
        js_sys::Reflect::set(&import_object, &"env".into(), env_object).unwrap();

        let promise = js_sys::WebAssembly::instantiate_buffer(&*wasm_bytes, import_object);
        let closure = Closure::wrap(Box::new(move |result| {
            let value = js_sys::Reflect::get(
                &result,
                &JsValue::from_str("instance")).unwrap();
            web_sys::console::log_1(&value);
            let exports = js_sys::Reflect::get(&value, &"exports".into()).unwrap();
            let factorial = js_sys::Reflect::get(&exports, &"factorial".into()).unwrap();
            let factorial = factorial.unchecked_into::<js_sys::Function>();
            let result = js_sys::Reflect::apply(&factorial, &JsValue::null(), &js_sys::Array::new()).unwrap();
            web_sys::console::log_1(&result);
        }) as Box<dyn FnMut(_)>);

        let _ = promise.then(&closure);

        // Note: This leaks memory, don't want to do this on every compliation.
        closure.forget();

        // js_sys::WebAssembly::compile()
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
