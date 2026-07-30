(lang dune 3.0)
(using cinaps 1.0)
(name lsp)

(implicit_transitive_deps false)

(license ISC)
(maintainers "Rudi Grinberg <me@rgrinberg.com>")
(authors
 "Andrey Popp <8mayday@gmail.com>"
 "Rusty Key <iam@stfoo.ru>"
 "Louis Roché <louis@louisroche.net>"
 "Oleksiy Golovko <alexei.golovko@gmail.com>"
 "Rudi Grinberg <me@rgrinberg.com>"
 "Sacha Ayoun <sachaayoun@gmail.com>"
 "cannorin <cannorin@gmail.com>"
 "Ulugbek Abdullaev <ulugbekna@gmail.com>"
 "Thibaut Mattio <thibaut.mattio@gmail.com>"
 "Max Lantas <mnxndev@outlook.com>")

(source (github ocaml/ocaml-lsp))

(generate_opam_files true)

(package
 (name lsp)
 (synopsis "LSP protocol implementation in OCaml")
 (description "
Implementation of the LSP protocol in OCaml. It is designed to be as portable as
possible and does not make any assumptions about IO.
")
 (depends
  (jsonrpc (= :version))
  yojson
  (ppx_yojson_conv_lib (>= "v0.14"))
  (cinaps :with-test)
  (ppx_expect (and (>= v0.17.0) :with-test))
  (uutf (>= 1.0.2))
  (odoc :with-doc)
  (ocaml (>= 4.14))
  (ppx_yojson_conv :with-dev-setup)))

(package
 (name ocaml-lsp-server)
 (synopsis "LSP Server for OCaml")
 (description "An LSP server for OCaml.")
 (depends
  yojson
  (base (>= v0.16.0))
  (lsp (= :version))
  (jsonrpc (= :version))
  (re (>= 1.5.0))
  (ppx_yojson_conv_lib (>= "v0.14"))
  (dune-rpc (>= 3.22.0))
  (chrome-trace (>= 3.3.0))
  dyn
  (stdune (>= 3.22.0))
  (fiber (and (>= 3.1.1) (< 4.0.0)))
  (ocaml (and (>= 5.5) (< 5.6)))
  xdg
  ordering
  (dune-build-info (>= 3.22.0))
  spawn
  astring
  camlp-streams
  (ppx_expect (and (>= v0.17.0) :with-test))
  (ocamlformat (and :with-test (= 0.28.1)))
  (ocamlc-loc (>= 3.7.0))
  (pp (>= 1.1.2))
  (csexp (>= 1.5))
  (ocamlformat-rpc-lib (>= 0.21.0))
  (odoc :with-doc)
  (merlin-lib (and (>= 5.8) (< 5.9)))
  (ocaml-index :post)
  (ppx_yojson_conv :with-dev-setup)))

(package
 (name jsonrpc)
 (synopsis "Jsonrpc protocol implementation")
 (description "See https://www.jsonrpc.org/specification")
 (depends
  (yojson (>= 2.0))
  (ocaml (>= 4.08))
  (odoc :with-doc)))
