// Example Coccinelle-for-Rust semantic patch.
// This is illustrative and may need adjustment for real codex code.
@@
expression tcx, arg;
@@
- tcx.type_of(arg)
+ tcx.bound_type_of(arg).subst_identity()
