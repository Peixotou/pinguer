# Interoperability

Status: **Proposal.** v0.1 programs cannot call foreign code. Internally,
compiled v0.1 programs call the platform C library for output and process
termination; this is an implementation detail, not a user-visible feature.
Implementation: not started.

## 1. Calling C (Proposal)

```pinguer
extern "C" {
    func strlen(s: *const u8) -> usize
}

func length(text: CString) -> usize {
    unsafe {
        // SAFETY: `text` owns a valid null-terminated buffer.
        return strlen(text.asPtr())
    }
}
```

* Foreign functions are declared in `extern "C"` blocks and use the platform
  C calling convention.
* Calling a foreign function is an unsafe operation: the compiler cannot check
  what the callee does with pointers.
* Only types with a defined C layout may cross the boundary: integers,
  floats, `bool` (as C `_Bool`), raw pointers, and structs marked with a C
  layout attribute (`@repr(C)`, **Proposal**).
* Pinguer functions can be exported to C with `extern "C" func` and a stable
  symbol name.

## 2. Binding generation (Proposal)

A tool (`pinguer bindgen`) will generate `extern` declarations from C headers
using libclang, so that bindings are not written by hand.

## 3. Embedding (Proposal)

Lua's strength is embeddability. Pinguer can approach it from the other side:
a Pinguer library compiled with C exports can be linked into C, C++, Rust or
other hosts. An embedding API for loading Pinguer code dynamically (with a
JIT or bytecode backend) is a separate, later proposal.

## 4. Other languages

* **C++:** through C wrappers only. Direct C++ interop (templates, classes)
  is out of scope.
* **Rust:** through the C ABI.
* **WebAssembly hosts:** through the planned WebAssembly backend and the
  component model, evaluated when that backend exists.

## 5. Unresolved questions

1. Spelling of raw pointer types (`*const T` / `*mut T` versus named types
   such as `RawPtr<T>`).
2. How strings cross the boundary (`CString`/`CStr` library types are
   expected).
3. Whether callbacks from C into Pinguer may trap, and what happens if they
   do (trapping across a C frame must not unwind into C).
