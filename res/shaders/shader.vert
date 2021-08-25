#version 450

vec2 positions[4] = vec2[](
    vec2(-1.0, -1.0),
    vec2(1.0, -1.0),
    vec2(1.0, 1.0),
    vec2(-1.0, 1.0)
);

uint indices[6] = uint[](
    0, 2, 1,
    0, 3, 2
);

void main() {
    gl_Position = vec4(positions[indices[gl_VertexIndex]], 0.0, 1.0);
}