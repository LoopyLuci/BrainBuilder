#lang racket
(require racket/match)
(provide symbolic-diff)

;; A simple symbolic differentiator for arithmetic expressions.
;; Extensible to handle tensor operations.
;; NOTE: the original blueprint declared `#lang racket/base` but used `match`,
;; which racket/base does not provide on its own — needs `(require racket/match)`,
;; included above (or switch the #lang line to full `racket`, done here).
(define (symbolic-diff expr var)
  (match expr
    [(? number?) 0]
    [(? symbol?) (if (eq? expr var) 1 0)]
    [`(+ ,a ,b) `(+ ,(symbolic-diff a var) ,(symbolic-diff b var))]
    [`(- ,a ,b) `(- ,(symbolic-diff a var) ,(symbolic-diff b var))]
    [`(* ,a ,b) `(+ (* ,a ,(symbolic-diff b var)) (* ,b ,(symbolic-diff a var)))]
    [`(/ ,a ,b) `(/ (- (* ,(symbolic-diff a var) ,b) (* ,a ,(symbolic-diff b var))) (* ,b ,b))]
    [`(exp ,a) `(* (exp ,a) ,(symbolic-diff a var))]
    [`(log ,a) `(* (/ 1 ,a) ,(symbolic-diff a var))]
    [`(sin ,a) `(* (cos ,a) ,(symbolic-diff a var))]
    [`(cos ,a) `(* (- (sin ,a)) ,(symbolic-diff a var))]
    [_ (error 'symbolic-diff "unsupported expression: ~a" expr)]))
