(ns brainbuilder.config)

(defmacro defgraph
  "Define a reusable graph configuration."
  [name & specs]
  `(def ~name {:graph/name '~name :specs [~@specs]}))
