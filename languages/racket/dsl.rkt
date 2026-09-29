#lang racket/base
(provide defcomponent)

(define-syntax-rule (defcomponent name body ...)
  (begin
    (printf "Registering component: ~a\n" 'name)
    (define name '(body ...))))
