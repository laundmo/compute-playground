#import bevy_pbr::utils

@group(0) @binding(0)
var texture: texture_storage_2d<rgba8unorm, read_write>;

struct ShaderParams {
    extents: vec4<f32>,
    size: vec2<f32>
}

@group(0) @binding(1)
var<uniform> params: ShaderParams;

struct Complex {
    re: f32,
    im: f32
}

fn cdiv(ca: Complex, cb: Complex) -> Complex {
    var result = Complex(0.0, 0.0);
    var a: f32 = (cb.re * cb.re + cb.im * cb.im);
    result.re = (ca.re * cb.re + ca.im * cb.im) / a;
    result.im = (ca.im * cb.re - ca.re * cb.im) / a;
    return result;
}

fn csub(ca: Complex, cb: Complex) -> Complex {
    var result = Complex(0.0, 0.0);
    result.re = ca.re - cb.re;
    result.im = ca.im - cb.im;
    return result;
}

fn cabs(c: Complex) -> f32 {
    return sqrt(c.re * c.re + c.im * c.im);
}   

fn cpoly(c: Complex) -> Complex {
    var result = Complex(0.0, 0.0);
    result.re = c.re * c.re * c.re - 3.0 * c.re * c.im * c.im - 1.0;
    result.im = 3.0 * c.re * c.re * c.im - c.im * c.im * c.im;
    return result;
}
fn cderivedPoly(c: Complex) -> Complex {
    var result = Complex(0.0, 0.0);
    result.re = 3.0 * c.re * c.re - 3.0 * c.im * c.im;
    result.im = 6.0 * c.im * c.re;
    return result;
}

fn newtonStep(c: Complex) -> Complex {
    return csub(c, cdiv(cpoly(c), cderivedPoly(c)));
}


fn classifyRoot(c: Complex, prec: f32) -> u32 {
    let root1 = Complex(1.0, 0.0);
    let root2 = Complex(-0.5, sqrt(3.0) / 2.0);
    let root3 = Complex(-0.5, -sqrt(3.0) / 2.0);

    if cabs(csub(c, root1)) < prec {
        return 1u;
    } else if cabs(csub(c, root2)) < prec {
        return 2u;
    } else if cabs(csub(c, root3)) < prec {
        return 3u;
    } else {
        return 0u;
    }
}

struct NewtonResult {
    root: u32,
    iter: u32
}

fn newton(c: Complex, prec: f32, maxiter: u32) -> NewtonResult {
    var c = c;
    var r: u32 = 0u;
    for (var i = 0u; i < maxiter; i++) {
        c = newtonStep(c);
        r = classifyRoot(c, prec);
        if r != 0u {
            return NewtonResult(r, i);
        }
    }
    return NewtonResult(0u, maxiter);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return (1.0 - t) * a + t * b;
}

fn inv_lerp(a: f32, b: f32, v: f32) -> f32 {
    return (v - a) / (b - a);
}

fn remap(i_min: f32, i_max: f32, o_min: f32, o_max: f32, v: f32) -> f32 {
    return lerp(o_min, o_max, inv_lerp(i_min, i_max, v));
}


@compute @workgroup_size(8,8,1) 
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
}

@compute @workgroup_size(8,8,1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(invocation_id.xy);
    var x_coord = remap(0.0, params.size.x, params.extents.x, params.extents.y, f32(location.x));
    var y_coord = remap(0.0, params.size.y, params.extents.w, params.extents.z, f32(location.y));
//    textureStore(texture, location, vec4<f32>(x_coord, y_coord, 0.0, 1.0));

    var z = Complex(x_coord, y_coord);
    var newton_res = newton(z, 0.0001, 80u);
    var color: vec4<f32>;
    var i = newton_res.iter;
    var c = newton_res.root;
    var v = 1.0 / (0.2 * f32(i) + 1.0);
    color = vec4<f32>(hsv2rgb(f32(c) / 3.0, 1.0 - v / 2.0, v), 1.0);
    textureStore(texture, location, color);
}