@group(0) @binding(0) var<storage, read> input: array<u8>;
@group(0) @binding(1) var<storage, read_write> output: array<f32,3>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x * 3u32;
    if (idx == 0u32) {
        var r: u32 = 0u32;
        var g: u32 = 0u32;
        var b: u32 = 0u32;
        var i: u32 = 0u32;
        loop {
            if (i >= arrayLength(&input)) { break; }
            r = r + u32(input[i]);
            g = g + u32(input[i+1u]);
            b = b + u32(input[i+2u]);
            i = i + 3u32;
        }
        let total: f32 = f32(arrayLength(&input)) / 3.0 * 255.0;
        output[0] = f32(r) / total;
        output[1] = f32(g) / total;
        output[2] = f32(b) / total;
    }
}
