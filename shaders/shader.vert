#version 450
#extension GL_EXT_shader_explicit_arithmetic_types_int8 : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

layout(binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
} ubo;

layout(push_constant) uniform PushConstants {
    i64vec3 model;
} pcs;


layout(location = 0) in u8vec3 pos;
layout(location = 0) out vec3 fragColor;


void main() {
    gl_Position = ubo.proj * ubo.view *  vec4(pcs.model * 32 + pos, 1.0);
    fragColor = vec3(1., 1., 1.);
}

