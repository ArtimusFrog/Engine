// Himmel, Standard-Material (Sonne mit Schatten, Umgebungslicht, Nebel), Wasser, Laub
// und leuchtende Objekte. Alle Farben in linearem Licht, am Ende filmisches Tone-Mapping.

struct Globals {
    view_proj: mat4x4<f32>,
    // Sicht der Sonne für die Schattenkarte
    light_view_proj: mat4x4<f32>,
    // Vom Bildschirm zurück in die Welt (für den Himmel)
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // Richtung *zur* Sonne, normiert.
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    sky_ambient: vec4<f32>,
    ground_ambient: vec4<f32>,
    // rgb = Horizontfarbe (auch Nebel), a = Nebeldichte
    fog: vec4<f32>,
    // x = Texelgröße der Schattenkarte, y = Normalen-Versatz, z = Zeit in Sekunden
    shadow_params: vec4<f32>,
    // rgb = Himmelsfarbe oben, a = Belichtung
    zenith: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(1) @binding(0) var albedo_texture: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;

const MAT_WATER: f32 = 1.0;
const MAT_FOLIAGE: f32 = 2.0;
const MAT_EMISSIVE: f32 = 3.0;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) vertex_color: vec3<f32>,
    // Instanzdaten: Modellmatrix (Spalten), Normalenmatrix (Spalten), Farbe, Material
    @location(3) m0: vec4<f32>,
    @location(4) m1: vec4<f32>,
    @location(5) m2: vec4<f32>,
    @location(6) m3: vec4<f32>,
    @location(7) n0: vec3<f32>,
    @location(8) n1: vec3<f32>,
    @location(9) n2: vec3<f32>,
    @location(10) color: vec4<f32>,
    @location(11) material: vec4<f32>,
    @location(12) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) @interpolate(flat) material: vec4<f32>,
    @location(4) uv: vec2<f32>,
};

fn time() -> f32 {
    return g.shadow_params.z;
}

// Wellenhöhe des Wassers und ihre Ableitungen (dh/dx, dh/dz).
fn water_wave(p: vec2<f32>) -> vec3<f32> {
    let t = time();
    let a = p.x * 0.35 + t * 1.1;
    let b = p.y * 0.27 - t * 0.9;
    let c = (p.x + p.y) * 0.8 + t * 2.0;
    let h = sin(a) * 0.12 + sin(b) * 0.10 + sin(c) * 0.03;
    let dx = cos(a) * 0.35 * 0.12 + cos(c) * 0.8 * 0.03;
    let dz = cos(b) * 0.27 * 0.10 + cos(c) * 0.8 * 0.03;
    return vec3<f32>(h, dx, dz);
}

// Wind: verschiebt Laub oben stärker als unten.
fn wind_offset(world: vec3<f32>, local_height: f32, strength: f32) -> vec3<f32> {
    let t = time();
    let phase = t * 1.7 + world.x * 0.35 + world.z * 0.27;
    let gust = 0.6 + 0.4 * sin(t * 0.37 + world.x * 0.05);
    let h = max(local_height, 0.0) * strength * gust;
    return vec3<f32>(sin(phase) * h, 0.0, cos(phase * 0.8) * h * 0.6);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    let normal_matrix = mat3x3<f32>(in.n0, in.n1, in.n2);
    var world = (model * vec4<f32>(in.position, 1.0)).xyz;
    if (in.material.x == MAT_FOLIAGE) {
        world += wind_offset(world, in.position.y, in.material.y);
    } else if (in.material.x == MAT_WATER) {
        world.y += water_wave(world.xz).x;
    }

    var out: VertexOut;
    out.clip = g.view_proj * vec4<f32>(world, 1.0);
    out.world_pos = world;
    out.normal = normal_matrix * in.normal;
    out.color = in.vertex_color * in.color.rgb;
    out.material = in.material;
    out.uv = in.uv;
    return out;
}

// Schatten-Durchgang: nur Tiefe aus Sicht der Sonne (Wind bewegt auch den Schatten).
@vertex
fn vs_shadow(in: VertexIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    var world = (model * vec4<f32>(in.position, 1.0)).xyz;
    if (in.material.x == MAT_FOLIAGE) {
        world += wind_offset(world, in.position.y, in.material.y);
    }
    if (in.material.x == MAT_WATER) {
        // Wasser wirft keinen Schatten.
        return vec4<f32>(0.0, 0.0, -1.0, 1.0);
    }
    return g.light_view_proj * vec4<f32>(world, 1.0);
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

// Himmelsfarbe in Blickrichtung `dir` (normiert), inklusive Sonnenscheibe.
fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    let horizon = g.fog.rgb;
    let up = max(dir.y, 0.0);
    var color = mix(horizon, g.zenith.rgb, pow(up, 0.45));
    if (dir.y < 0.0) {
        color = mix(horizon, horizon * 0.8, min(-dir.y * 3.0, 1.0));
    }
    let s = max(dot(dir, g.sun_dir.xyz), 0.0);
    color += g.sun_color.rgb * (pow(s, 1200.0) * 30.0 + pow(s, 16.0) * 0.35 + pow(s, 3.0) * 0.08);
    return color;
}

// Filmisches Tone-Mapping (ACES-Näherung): helle Stellen laufen weich aus statt auszubrennen.
fn tonemap(color: vec3<f32>) -> vec3<f32> {
    let x = color * g.zenith.a;
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_fog(color: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let to_point = world_pos - g.camera_pos.xyz;
    let dist = length(to_point);
    // Nebel wird mit der Höhe dünner, damit Berggipfel klar bleiben.
    let height_falloff = exp(-max(world_pos.y, 0.0) * 0.02);
    let fog = 1.0 - exp(-dist * g.fog.a * height_falloff);
    return mix(color, sky_color(to_point / max(dist, 0.001)), fog);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Textur vor allen Verzweigungen lesen (WGSL verlangt einheitlichen Kontrollfluss).
    let albedo = in.color * textureSample(albedo_texture, albedo_sampler, in.uv).rgb;
    let kind = in.material.x;
    var n = normalize(in.normal);
    let view = normalize(g.camera_pos.xyz - in.world_pos);

    if (kind == MAT_WATER) {
        let wave = water_wave(in.world_pos.xz);
        n = normalize(vec3<f32>(-wave.y, 1.0, -wave.z));
        let fresnel = pow(1.0 - max(dot(n, view), 0.0), 4.0);
        let deep = vec3<f32>(0.015, 0.09, 0.16);
        let shallow = vec3<f32>(0.03, 0.28, 0.33) * in.color;
        let body = mix(shallow, deep, clamp(1.0 - dot(n, view), 0.0, 1.0));
        let reflection = sky_color(reflect(-view, n));
        let visibility = sun_visibility(in.world_pos, vec3<f32>(0.0, 1.0, 0.0));
        let sparkle = pow(max(dot(reflect(-g.sun_dir.xyz, n), view), 0.0), 180.0) * 6.0 * visibility;
        var color = mix(body * (0.6 + 0.4 * visibility), reflection, 0.15 + fresnel * 0.75) + g.sun_color.rgb * sparkle;
        color = apply_fog(color, in.world_pos);
        return vec4<f32>(tonemap(color), 1.0);
    }

    // Blätter sind oft von hinten zu sehen: dann die Rückseite beleuchten.
    if (kind == MAT_FOLIAGE && dot(n, view) < 0.0) {
        n = -n;
    }
    let n_dot_l = dot(n, g.sun_dir.xyz);
    var diffuse = max(n_dot_l, 0.0);
    if (n_dot_l > 0.0) {
        diffuse *= sun_visibility(in.world_pos, n);
    }
    let ambient = mix(g.ground_ambient.rgb, g.sky_ambient.rgb, n.y * 0.5 + 0.5);
    var color = albedo * (ambient + g.sun_color.rgb * diffuse);
    if (kind == MAT_EMISSIVE) {
        color += albedo * in.material.y;
    }
    color = apply_fog(color, in.world_pos);
    return vec4<f32>(tonemap(color), 1.0);
}

// ---------- Himmel: ein Dreieck, das den ganzen Bildschirm bedeckt ----------

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOut {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    let ndc = uv * 2.0 - 1.0;
    var out: SkyOut;
    out.clip = vec4<f32>(ndc, 1.0, 1.0);
    out.ndc = ndc;
    return out;
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4<f32> {
    let far = g.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(far.xyz / far.w - g.camera_pos.xyz);
    return vec4<f32>(tonemap(sky_color(dir)), 1.0);
}
