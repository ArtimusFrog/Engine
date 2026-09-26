// Hochskalieren der Welt (Renderauflösung unter 100 %) aufs Fenster, mit leichtem Nachschärfen,
// damit das Bild trotz geringerer Auflösung klar wirkt.

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;

struct BlitOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) index: u32) -> BlitOut {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: BlitOut;
    out.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_blit(in: BlitOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(scene));
    let d = 1.0 / size;
    let c = textureSampleLevel(scene, scene_sampler, in.uv, 0.0).rgb;
    let n = textureSampleLevel(scene, scene_sampler, in.uv + vec2<f32>(0.0, -d.y), 0.0).rgb;
    let s = textureSampleLevel(scene, scene_sampler, in.uv + vec2<f32>(0.0, d.y), 0.0).rgb;
    let e = textureSampleLevel(scene, scene_sampler, in.uv + vec2<f32>(d.x, 0.0), 0.0).rgb;
    let w = textureSampleLevel(scene, scene_sampler, in.uv + vec2<f32>(-d.x, 0.0), 0.0).rgb;
    // Je stärker verkleinert, desto mehr nachschärfen (Fenstergröße = Pixelposition / uv).
    let out_size = in.clip.xy / max(in.uv, vec2<f32>(1e-4));
    let amount = clamp((1.0 - size.x / max(out_size.x, 1.0)) * 0.9, 0.0, 0.3);
    // Kontrastadaptiv: an harten Kanten weniger, sonst entstehen helle Säume.
    let lo = min(min(min(n, s), min(e, w)), c);
    let hi = max(max(max(n, s), max(e, w)), c);
    let sharp = c + (c * 4.0 - n - s - e - w) * amount;
    return vec4<f32>(clamp(sharp, lo, hi), 1.0);
}
