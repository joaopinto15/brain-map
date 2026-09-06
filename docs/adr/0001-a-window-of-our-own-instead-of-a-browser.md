# A window of our own, instead of a browser

brain-map drew itself as a web page for its whole life: a loopback server, HTML and CSS,
and latterly Rust compiled to WebAssembly. We replaced all of it with a native window
built on `eframe`, because a page cannot be run without JavaScript — a browser has no way
to start a WebAssembly module except from a JS call, so wasm-bindgen's loader and Dioxus's
interpreter were always going to be there, and we wanted none of them.

## Considered options

- **Keep the page, drop Dioxus.** Would have removed the ~24 KB interpreter and left only
  wasm-bindgen's generated loader. Cheaper, but it does not reach zero, and the loader is
  the part we could never remove.
- **Hand-write the JS bridge instead of generating it.** Reaches a smaller total, but every
  DOM call becomes an import we maintain. That is *more* JavaScript we are responsible for,
  not less.
- **A native window.** The only option that reaches zero, because there is no page.

## Consequences

The dependency count went the wrong way. The rule was five direct dependencies and the
standard library for everything else; it is now four, but `eframe` pulls roughly three
hundred packages where the old stack pulled ten. The rule still governs code we write, and
no longer governs what is in the lock. That is the price, and it was paid knowingly.

Two things were lost. There is no browser fallback and no `--browser` flag, so a machine
with no display cannot show the graph at all; before, it could. And there are no devtools,
so a rendering problem is now debugged by reading the painter rather than by inspecting.

Two things came free. A note is no longer HTML — the renderer produces styled blocks, so
there is nothing to escape and no path by which note text could become markup. And the
split the HTTP API drew survives as the `Source` trait, which is the same six operations
with the transport taken out, so it did not have to be re-invented.
