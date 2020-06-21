use rhai::{Engine, EvalAltResult, Scope, RegisterFn, AST};
use crate::engine::{EMPTY, Particle, WorldView, Kind};
use rand::{thread_rng, Rng};
use rand::prelude::ThreadRng;
use std::collections::HashMap;

pub struct ScriptEngine {
    engine: Engine,
    scripts: HashMap<Kind, AST>,
}

impl ScriptEngine {
    pub(crate) fn new() -> Self {
        let mut engine = Engine::new();

        engine.register_type::<WorldView>();

        engine.register_fn("get", WorldView::get);
        engine.register_fn("set", WorldView::set);

        engine.register_type::<Particle>();

        engine.register_get("kind", Particle::get_kind);

        engine.register_type::<ThreadRng>();
        engine.register_fn("gen_bool", ThreadRng::gen_bool);

        let mut scripts = HashMap::new();

        let sand_script = engine.compile(r"
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
        ").unwrap();

        scripts.insert(Kind::Sand, sand_script);

        Self {
            engine,
            scripts,
        }
    }

    pub(crate) fn run(&mut self, view: WorldView, kind: Kind) -> Result<(), Box<EvalAltResult>> {
        let mut view = view.clone();

        let mut scope = Scope::new();
        let rng = thread_rng();

        scope.push_constant("KIND_SAND", Kind::Sand.value());
        scope.push_constant("KIND_EMPTY", Kind::Empty.value());
        scope.push_constant("KIND_WATER", Kind::Water.value());
        scope.push_constant("KIND_FIRE", Kind::Fire.value());
        scope.push_constant("KIND_PLANT", Kind::Plant.value());
        scope.push_constant("EMPTY", EMPTY);
        scope.push_constant("current", view.get(0, 0));
        scope.push_constant("view", view);
        scope.push("rng", rng);

        self.engine.eval_ast_with_scope(&mut scope, &self.scripts[&kind])?;

        Ok(())
    }
}
