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
    // Himmelskörper: xyz = Richtung, w = Sichtbarkeit
    sky_sun: vec4<f32>,
    sky_moon: vec4<f32>,
    // x = Sterne, y = Dämmerungsglühen, z = Anzahl Punktlichter
    sky_misc: vec4<f32>,
    // Punktlichter: je zwei Einträge (Position + Reichweite, Farbe)
    lights: array<vec4<f32>, 16>,
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
fn shadow_clip(in: VertexIn) -> vec4<f32> {
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

@vertex
fn vs_shadow(in: VertexIn) -> @builtin(position) vec4<f32> {
    return shadow_clip(in);
}

struct ShadowOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Schatten für Ausschnitt-Meshes (Blätter mit Löchern): die Textur entscheidet, wo Schatten fällt.
@vertex
fn vs_shadow_uv(in: VertexIn) -> ShadowOut {
    var out: ShadowOut;
    out.clip = shadow_clip(in);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_shadow_cutout(in: ShadowOut) {
    if (textureSample(albedo_texture, albedo_sampler, in.uv).a < 0.5) {
        discard;
    }
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

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

// Sterne: zufällig verteilte, leicht funkelnde Punkte am Himmel.
fn stars(dir: vec3<f32>) -> vec3<f32> {
    let p = dir * 220.0;
    let cell = floor(p);
    let h = hash3(cell);
    if (h < 0.9965) {
        return vec3<f32>(0.0);
    }
    let jitter = vec3<f32>(hash3(cell + 1.3), hash3(cell + 7.1), hash3(cell + 3.7)) - 0.5;
    let d = length(p - (cell + 0.5 + jitter * 0.6));
    let twinkle = 0.65 + 0.35 * sin(time() * (2.0 + h * 3.0) + h * 1000.0);
    let brightness = (1.0 - smoothstep(0.0, 0.35, d)) * twinkle * (h - 0.9965) / 0.0035 * 3.0;
    let tint = mix(vec3<f32>(0.75, 0.85, 1.0), vec3<f32>(1.0, 0.9, 0.75), hash3(cell + 9.9));
    return tint * brightness;
}

fn sun_tint() -> vec3<f32> {
    // Tief am Horizont orange, hoch am Himmel weiß.
    return mix(vec3<f32>(1.7, 0.75, 0.35), vec3<f32>(1.3, 1.1, 0.9), clamp(g.sky_sun.y * 3.0, 0.0, 1.0));
}

// Weicher Himmel ohne Sonnenscheibe, Mond und Sterne – auch die Farbe des Nebels.
fn sky_base(dir: vec3<f32>) -> vec3<f32> {
    let horizon = g.fog.rgb;
    let up = max(dir.y, 0.0);
    var color = mix(horizon, g.zenith.rgb, pow(up, 0.45));
    if (dir.y < 0.0) {
        color = mix(horizon, horizon * 0.8, min(-dir.y * 3.0, 1.0));
    }

    // Breiter Dunst in Richtung Sonne.
    let sun = g.sky_sun.xyz;
    let s = max(dot(dir, sun), 0.0);
    color += sun_tint() * g.sky_sun.w * pow(s, 3.0) * 0.08;

    // Abendrot bzw. Morgenrot rund um die Sonne, entlang des Horizonts.
    let flat_sun = normalize(vec3<f32>(sun.x, 0.0, sun.z) + vec3<f32>(0.0001, 0.0, 0.0));
    let toward = max(dot(dir, flat_sun), 0.0);
    let near_horizon = 1.0 - smoothstep(0.0, 0.45, abs(dir.y));
    color += vec3<f32>(1.0, 0.32, 0.1) * g.sky_misc.y * pow(toward, 3.0) * near_horizon * 0.9;
    return color;
}

// Himmelsfarbe in Blickrichtung `dir` (normiert): Verlauf, Sonne, Abendrot, Mond, Sterne.
fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    var color = sky_base(dir);

    // Sonnenscheibe mit Lichthof
    let s = max(dot(dir, g.sky_sun.xyz), 0.0);
    color += sun_tint() * g.sky_sun.w * (pow(s, 1200.0) * 30.0 + pow(s, 16.0) * 0.35);

    // Mond: scharfe Scheibe mit weichem Lichthof.
    let m = max(dot(dir, g.sky_moon.xyz), 0.0);
    color += vec3<f32>(0.85, 0.9, 1.0) * g.sky_moon.w * (smoothstep(0.99935, 0.9997, m) * 2.5 + pow(m, 80.0) * 0.12);

    if (g.sky_misc.x > 0.0 && dir.y > 0.0) {
        color += stars(dir) * g.sky_misc.x * smoothstep(0.0, 0.25, dir.y);
    }
    return color;
}

// Filmisches Tone-Mapping (ACES-Näherung): helle Stellen laufen weich aus statt auszubrennen.
fn tonemap(color: vec3<f32>) -> vec3<f32> {
    let x = color * g.zenith.a;
    let mapped = clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
    // ACES nimmt Farben etwas Sättigung – ein wenig zurückgeben, für eine fröhliche, satte Welt.
    let grey = vec3<f32>(dot(mapped, vec3<f32>(0.2126, 0.7152, 0.0722)));
    return clamp(mix(grey, mapped, 1.07), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Licht von Punktlichtern (Feuer, Laternen): weich zum Rand der Reichweite auslaufend.
fn point_lights(world_pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var total = vec3<f32>(0.0);
    let count = i32(g.sky_misc.z);
    for (var i = 0; i < count; i++) {
        let position = g.lights[i * 2];
        let color = g.lights[i * 2 + 1].rgb;
        let to_light = position.xyz - world_pos;
        let distance = length(to_light);
        let falloff = clamp(1.0 - distance / position.w, 0.0, 1.0);
        // Etwas "Umlicht", damit auch abgewandte Flächen nicht ganz schwarz sind.
        let facing = max(dot(n, to_light / max(distance, 0.001)), 0.0) * 0.8 + 0.2;
        total += color * facing * falloff * falloff;
    }
    return total;
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Nachtsicht: Im Dunkeln verlieren Farben an Sättigung und wirken bläulich.
fn night_grade(color: vec3<f32>) -> vec3<f32> {
    let night = g.sky_misc.x;
    let moonlit = vec3<f32>(luminance(color)) * vec3<f32>(0.55, 0.75, 1.35);
    return mix(color, moonlit, night * 0.65);
}

fn apply_fog(color: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let to_point = world_pos - g.camera_pos.xyz;
    let dist = length(to_point);
    // Nebel wird mit der Höhe dünner, damit Berggipfel klar bleiben.
    let height_falloff = exp(-max(world_pos.y, 0.0) * 0.02);
    let fog = 1.0 - exp(-dist * g.fog.a * height_falloff);
    return mix(color, sky_base(to_point / max(dist, 0.001)), fog);
}

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // Textur vor allen Verzweigungen lesen (WGSL verlangt einheitlichen Kontrollfluss).
    let texel = textureSample(albedo_texture, albedo_sampler, in.uv);
    // Ausschnitt-Masken (Blätter, Gräser): durchsichtige Stellen gar nicht zeichnen.
    if (texel.a < 0.5) {
        discard;
    }
    let albedo = in.color * texel.rgb;
    let kind = in.material.x;
    var n = normalize(in.normal);
    // Blattkarten tragen weiche Kugel-Normalen aus Blender – die gelten für beide Seiten.
    let leaves = kind == MAT_FOLIAGE && in.material.z > 0.5;
    // Beidseitige Flächen: von hinten gesehen zeigt die Normale zum Betrachter.
    if (!front && !leaves) {
        n = -n;
    }
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
        // Das Wasser selbst ist nur so hell wie das Licht, das auf es fällt.
        let light = clamp(luminance(g.sky_ambient.rgb) * 2.0 + luminance(g.sun_color.rgb) * 0.5, 0.04, 1.0);
        var color = mix(night_grade(body * light * (0.6 + 0.4 * visibility)), reflection, 0.15 + fresnel * 0.75) + g.sun_color.rgb * sparkle;
        color = apply_fog(color, in.world_pos);
        return vec4<f32>(tonemap(color), 1.0);
    }

    // Blätter sind oft von hinten zu sehen: dann die Rückseite beleuchten.
    if (kind == MAT_FOLIAGE && !leaves && dot(n, view) < 0.0) {
        n = -n;
    }
    let n_dot_l = dot(n, g.sun_dir.xyz);
    var diffuse = max(n_dot_l, 0.0);
    var translucent = vec3<f32>(0.0);
    if (leaves) {
        // Weiches Licht um die Krone herum; Schatten in der Krone nie ganz schwarz.
        let visibility = sun_visibility(in.world_pos, n);
        diffuse = clamp((n_dot_l + 0.55) / 1.55, 0.0, 1.0) * mix(0.4, 1.0, visibility);
        // Gegenlicht: Blätter leuchten durch, wenn die Sonne hinter ihnen steht.
        let behind = pow(max(dot(-view, g.sun_dir.xyz), 0.0), 3.0);
        translucent = g.sun_color.rgb * behind * 0.45 * visibility;
    } else if (n_dot_l > 0.0) {
        diffuse *= sun_visibility(in.world_pos, n);
    }
    let ambient = mix(g.ground_ambient.rgb, g.sky_ambient.rgb, n.y * 0.5 + 0.5);
    var color = night_grade(albedo * (ambient + g.sun_color.rgb * diffuse + translucent));
    // Warmes Licht von Feuer und Laternen – nicht entsättigt, es soll nachts leuchten.
    color += albedo * point_lights(in.world_pos, n);
    // Selbstleuchtendes bleibt farbig – nachts stechen Pilze, Kristalle und Funken heraus.
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
