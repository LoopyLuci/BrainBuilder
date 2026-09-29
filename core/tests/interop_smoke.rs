// Exercises the real Racket/Clojure subprocess bridges end-to-end. Ignored
// by default since they need `racket` and a JDK + resolved Clojure jars
// on PATH/~/.m2 respectively — run explicitly with `cargo test -- --ignored`
// once those toolchains are installed.
use brainbuilder_core::interop::clojure::ClojureBridge;
use brainbuilder_core::interop::racket::RacketEngine;

#[test]
#[ignore]
fn racket_symbolic_diff_roundtrip() {
    let engine = RacketEngine::default();
    let result = engine.eval("(* x x)", "x").expect("racket bridge failed");
    assert_eq!(result, "(+ (* x 1) (* x 1))");
}

#[test]
#[ignore]
fn clojure_defgraph_roundtrip() {
    let bridge = ClojureBridge::default();
    let result = bridge
        .eval_config("(defgraph my-graph :loss \"mse\")")
        .expect("clojure bridge failed");
    assert_eq!(result, "#'brainbuilder.config/my-graph");
}
