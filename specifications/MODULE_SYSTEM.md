# Module System

Status: section 1 is **Normative (v0.1)**; §2.3 (visibility) is **Planned**;
the rest is **Proposal** or **Unresolved** as marked.
Implementation: not started.

## 1. Imports in v0.1 (Normative v0.1)

A v0.1 program is a single file. It may import modules of the standard
library:

```pinguer
import std.io

func main() {
    io.println("Hello, Pinguer!")
}
```

* `import a.b.c` makes the module available under its last path component,
  `c`, in the whole file.
* Only modules that exist in the v0.1 standard library may be imported
  ([STANDARD_LIBRARY.md](STANDARD_LIBRARY.md)); in v0.1 that is `std.io`.
  Importing anything else is an error.
* Importing the same module twice, or two modules with the same last
  component, is an error.
* A module name may be used only as the left operand of member access
  (`io.println`). It is not a value.
* An imported module name and a top-level declaration may not have the same
  name.

## 2. Modules and packages (Proposal)

### 2.1 Terminology

| Term | Meaning |
|---|---|
| **Module** | A namespace. Each source file is a module; a directory may group modules. |
| **Package** | A unit of distribution and versioning, described by a manifest, containing one or more modules. |
| **Target** | A buildable artifact of a package: an executable or a library. |

### 2.2 File-based modules

The module path follows the directory structure under the package's `src/`
directory, as in Go, Java and Rust 2018: `src/net/http.pgr` is module
`net.http` of its package. There is no separate module declaration file.

**Unresolved (C11), with recommendation:** a `module` declaration at the top
of a file and an `export` keyword were proposed. The recommendation is to
adopt **neither**:

* `module name` would duplicate information already given by the file path;
  the two could disagree, which needs an extra error and an extra rule for
  which one wins.
* `export` would overlap with `public` ([ADR-0012](../docs/adr/0012-visibility-public-and-private.md)).
  Exporting a symbol for other languages is handled by the `@export`
  attribute ([METAPROGRAMMING.md §3](METAPROGRAMMING.md#3-attributes-planned)).
* Re-exporting a declaration from another module (library facades) is a
  separate **Proposal**: `public import a.b.C`.

### 2.3 Visibility (Planned)

Decision: [ADR-0012](../docs/adr/0012-visibility-public-and-private.md).

| Modifier | Visible to |
|---|---|
| *(none)* | The declaring module only. |
| `private` | Same as no modifier; allowed for explicitness, no warning. |
| `public` | Every module and package that can import the declaring module. |

* Applies to top-level items, struct fields, methods and interfaces. Enum
  variants and interface members have the visibility of their enclosing
  declaration.
* A `public` declaration's signature may not mention a non-public type
  (error), so that importers can always name the types they receive.
* At most one modifier per declaration; `public private` is an error.
* In v0.1 (single-file programs) visibility modifiers have no meaning; the
  parser reports them as reserved for future use.
* **Unresolved:** a package-internal level (`internal`) and read-only fields
  (`readonly`). Neither word is reserved.
* **Rejected:** `protected`, which only has meaning with inheritance
  ([ADR-0011](../docs/adr/0011-composition-and-interfaces-without-class-inheritance.md)).

### 2.4 Import forms

```pinguer
import std.io                  // module, accessed as io.println
import std.collections.List    // single declaration
import net.http as web         // renaming
```

Glob imports (`import std.io.*`) are deliberately excluded: they make it
impossible to tell where a name comes from without tooling and make adding a
declaration to a library a breaking change.

**Unresolved:** how to distinguish a module of the current package from a
module of a dependency with the same name (candidates: a reserved `package`
root, or dependency names declared in the manifest taking precedence with an
error on conflict).

### 2.5 Circular imports

Modules within a package may import each other cyclically; the compiler
resolves the whole package at once. Packages may not depend on each other
cyclically.

### 2.6 Package manifest and lockfile

* Manifest: `pinguer.toml` (name, version, edition, dependencies, targets).
* Lockfile: `pinguer.lock`, recording the exact resolved version and a
  content hash of every dependency. Builds verify hashes.
* Versions follow Semantic Versioning. Resolution prefers a single version of
  each package per major version.
* **Unresolved:** registry design, namespacing of package names, and whether
  packages may run code at build time (see
  [SECURITY_MODEL.md](SECURITY_MODEL.md#4-supply-chain)).

### 2.7 Editions

A package declares the language edition it is written in. Editions may change
syntax (for example new keywords) while packages of different editions still
link together, as in Rust.
