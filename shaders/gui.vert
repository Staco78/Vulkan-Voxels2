#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 out_uv;

layout(binding = 0) uniform Uniform
{
    vec2 screen_size;
};

vec3 srgb_to_linear(vec3 srgb)
{
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 lower = srgb / vec3(12.92);
    vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    return mix(higher, lower, cutoff);
}

void main()
{
    gl_Position = vec4(
        2.0 * pos.x / screen_size.x - 1.0,
        2.0 * pos.y / screen_size.y - 1.0,
        0.0,
        1.0);
    fragColor = vec4(srgb_to_linear(color.rgb), color.a);
    out_uv = uv;
}