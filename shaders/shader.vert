#version 450
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

layout(binding = 0) uniform UniformBufferObject
{
    mat4 view;
    mat4 proj;
}
ubo;

layout(push_constant) uniform PushConstants
{
    i64vec3 model;
}
pcs;

layout(location = 0) in uint data;

layout(location = 0) out vec3 fragColor;

void main()
{
    ivec3 pos = ivec3(data & 63, (data >> 6) & 63, (data >> 12) & 63);
    uint face_light = 3 * ((data >> 18) & 3) + 4;
    gl_Position = ubo.proj * ubo.view * vec4(pcs.model * 32 + pos, 1.0);
    fragColor = vec3(1., 1., 1.) * (float(face_light) / 10.0);
}