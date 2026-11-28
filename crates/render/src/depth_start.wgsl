// The scene pass's depth before anything is drawn into it: the depth the
// prepass drew, one representable step farther (renderer/depth_start.rs),
// and on a cut-out's pixel that depth itself.
//
// A solid item's pixel that the prepass kept is then still nearer than
// what's there, so it is drawn, and writes its own depth: a second
// surface at exactly that depth fails the test, as it does when the depth
// starts at the far plane. Whatever is farther by a step or more fails
// before it is lit.

@group(0) @binding(0) var prepass_depth: texture_depth_2d;
// The prepass's normals, whose w says which pixels are a cut-out's
// (shader.wgsl's `fs_prepass`).
@group(0) @binding(1) var prepass_normals: texture_2d<f32>;

// Must match `PREPASS_STANDARD_CUT_OUT` and `PREPASS_CHARACTER_CUT_OUT` in
// shader.wgsl: the standard material's cut-outs, which the scene pass
// lights where their depth equals what's there.
const PREPASS_STANDARD_CUT_OUT: f32 = 0.85;
const PREPASS_CHARACTER_CUT_OUT: f32 = 0.625;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // One triangle over the whole target.
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

// The smallest depth this writes: the smallest normal float, so a GPU that
// rounds smaller ones to nothing can't make it 0, which nothing is nearer
// than.
const NEAREST: f32 = 1.1754944e-38;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @builtin(frag_depth) f32 {
    let depth = textureLoad(prepass_depth, vec2<i32>(position.xy), 0);
    // Nothing drawn there: the far plane, as a cleared depth has.
    if depth >= 1.0 {
        return 1.0;
    }
    // A cut-out's pixel keeps the prepass's depth exactly: the scene pass
    // lights cut-outs only where their depth equals what's there (as it did
    // over the depth the cut-outs' fill drew, which is this one), so they
    // need no fill of their own.
    let flag = textureLoad(prepass_normals, vec2<i32>(position.xy), 0).w;
    if abs(flag - PREPASS_STANDARD_CUT_OUT) < 0.01
        || abs(flag - PREPASS_CHARACTER_CUT_OUT) < 0.01 {
        return depth;
    }
    // The next float up: a positive float's bits, read as a whole number,
    // count the floats in order.
    return max(bitcast<f32>(bitcast<u32>(depth) + 1u), NEAREST);
}
