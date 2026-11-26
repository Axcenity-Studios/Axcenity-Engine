// How much cloud there is above the level point `xz`, worked out from the
// noise (clouds.wgsl's `cloud_sum`) where it's asked: every lit surface's
// cloud shadow, the fog's, and the sky pass's clouds while they aren't
// traced. Goes after clouds.wgsl in every shader that includes it; the
// traced clouds (cloud_trace.wgsl) have their own, which reads a texture.
fn cloud_fill(clouds: Clouds, xz: vec2<f32>) -> f32 {
    return cloud_cut(clouds, cloud_sum((xz - clouds.offset.xy) / CLOUD_SCALE));
}
