;; Subprocess protocol for `core/src/interop/clojure.rs`: `brainbuilder_config.clj`
;; declares `(ns brainbuilder.config)` but its filename doesn't follow
;; Clojure's namespace->path convention (it'd need to live at
;; `brainbuilder/config.clj` for `require` to find it), so this loads it
;; directly with `load-file` instead. Reads one line from stdin — a full
;; Clojure form as text, e.g. `(defgraph my-graph :loss "mse")` — evaluates
;; it in the `brainbuilder.config` namespace, and writes the printed result
;; as one line to stdout, or `ERROR: <message>`.
(load-file (str (System/getProperty "brainbuilder.config.path")))
(in-ns 'brainbuilder.config)

(let [input (read-line)]
  (if (nil? input)
    (println "ERROR: no input")
    (try
      (let [form (read-string input)
            result (eval form)]
        (println (str (pr-str result))))
      (catch Exception e
        (println (str "ERROR: " (.getMessage e)))))))
