#lang racket/base
;; Subprocess protocol for `core/src/interop/racket.rs`: reads two lines from
;; stdin (1: a racket-readable expression, 2: the variable name to
;; differentiate with respect to) and writes one line to stdout — either the
;; resulting expression's printed form, or `ERROR: <message>`. Deliberately
;; avoids the `json` package: this racket install (via scoop) doesn't ship it
;; as a collection and `raco pkg install json` has no network-reachable
;; catalog entry for it in this environment, so a plain line-based protocol
;; is what's actually buildable here. One request per process (Rust
;; spawns/tears down the subprocess per call).
(require "symbolic_ad.rkt")

(define expr-line (read-line (current-input-port) 'any))
(define var-line (read-line (current-input-port) 'any))

(cond
  [(or (eof-object? expr-line) (eof-object? var-line))
   (displayln "ERROR: expected two lines of input (expr, var)")]
  [else
   (with-handlers ([exn:fail? (lambda (e) (displayln (format "ERROR: ~a" (exn-message e))))])
     (define expr (read (open-input-string expr-line)))
     (define var (string->symbol var-line))
     (define result (symbolic-diff expr var))
     (displayln (format "~a" result)))])
