// Standard-Material: Sonnenlicht (Lambert), Himmel/Boden-Umgebungslicht und Nebel.

struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // Richtung *zur* Sonne, normiert.
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    sky_ambient: vec4<f32>,
    ground_ambient: vec4<f32>,
    // rgb = Nebelfarbe, a = Dichte
    fog: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

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

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    let normal_matrix = mat3x3<f32>(in.n0, in.n1, in.n2);
    let world = model * vec4<f32>(in.position, 1.0);

    var out: VertexOut;
    out.clip = g.view_proj * world;
    out.world_pos = world.xyz;
    out.normal = normal_matrix * in.normal;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let diffuse = max(dot(n, g.sun_dir.xyz), 0.0);
    let ambient = mix(g.ground_ambient.rgb, g.sky_ambient.rgb, n.y * 0.5 + 0.5);
    var color = in.color.rgb * (ambient + g.sun_color.rgb * diffuse);

    let dist = length(in.world_pos - g.camera_pos.xyz);
    let fog = 1.0 - exp(-dist * g.fog.a);
    color = mix(color, g.fog.rgb, fog);

    return vec4<f32>(color, 1.0);
}
