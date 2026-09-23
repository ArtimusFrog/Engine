// Standard-Material: Sonnenlicht (Lambert) mit Schatten, Himmel/Boden-Umgebungslicht und Nebel.

struct Globals {
    view_proj: mat4x4<f32>,
    // Sicht der Sonne für die Schattenkarte
    light_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // Richtung *zur* Sonne, normiert.
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    sky_ambient: vec4<f32>,
    ground_ambient: vec4<f32>,
    // rgb = Nebelfarbe, a = Dichte
    fog: vec4<f32>,
    // x = Texelgröße der Schattenkarte, y = Normalen-Versatz in Metern
    shadow_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    // Instanzdaten: Modellmatrix (Spalten), Normalenmatrix (Spalten), Farbe
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
    @location(6) n0: vec3<f32>,
    @location(7) n1: vec3<f32>,
    @location(8) n2: vec3<f32>,
    @location(9) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

fn model_matrix(in: VertexIn) -> mat4x4<f32> {
    return mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let normal_matrix = mat3x3<f32>(in.n0, in.n1, in.n2);
    let world = model_matrix(in) * vec4<f32>(in.position, 1.0);

    var out: VertexOut;
    out.clip = g.view_proj * world;
    out.world_pos = world.xyz;
    out.normal = normal_matrix * in.normal;
    out.color = in.color;
    return out;
}

// Schatten-Durchgang: nur Tiefe aus Sicht der Sonne.
@vertex
fn vs_shadow(in: VertexIn) -> @builtin(position) vec4<f32> {
    return g.light_view_proj * model_matrix(in) * vec4<f32>(in.position, 1.0);
}

// 1 = voll beleuchtet, 0 = im Schatten. Weiche Kanten durch 3×3-Mittelung (PCF).
fn sun_visibility(world_pos: vec3<f32>, n: vec3<f32>) -> f32 {
    // Etwas entlang der Normale versetzen, sonst beschattet sich die Fläche selbst ("Shadow Acne").
    let offset_pos = world_pos + n * g.shadow_params.y;
    let light_clip = g.light_view_proj * vec4<f32>(offset_pos, 1.0);
    let ndc = light_clip.xyz / light_clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0) {
        return 1.0; // außerhalb der Schattenkarte
    }

    var lit = 0.0;
    let texel = g.shadow_params.x;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let sample_uv = uv + vec2<f32>(f32(x), f32(y)) * texel;
            lit += textureSampleCompareLevel(shadow_map, shadow_sampler, sample_uv, ndc.z);
        }
    }
    return lit / 9.0;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let n_dot_l = dot(n, g.sun_dir.xyz);
    var diffuse = max(n_dot_l, 0.0);
    if (n_dot_l > 0.0) {
        diffuse *= sun_visibility(in.world_pos, n);
    }
    let ambient = mix(g.ground_ambient.rgb, g.sky_ambient.rgb, n.y * 0.5 + 0.5);
    var color = in.color.rgb * (ambient + g.sun_color.rgb * diffuse);

    let dist = length(in.world_pos - g.camera_pos.xyz);
    let fog = 1.0 - exp(-dist * g.fog.a);
    color = mix(color, g.fog.rgb, fog);

    return vec4<f32>(color, 1.0);
}
