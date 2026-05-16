# level-1b compiler source

Source loading owns file IO, include path resolution, line mapping, doc comment
capture, namespace/file documentation, and `compile_if` header filtering.

Only `//` ordinary comments and `///` doc comments exist. Block comments are not
part of the language surface.

`#[doc(path="...")] namespace name` attaches external Markdown documentation to
the namespace surface. The scanner must retain source spans for diagnostics and
documentation output.
