#version 450

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outColorDebug;

layout(std140, binding = 0) uniform Uniforms {
    vec3 view_pos;
    vec3 view_dir;
    vec3 view_up;
    vec3 view_right;
    float fov;
    uint width;
    uint height;
    float octree_size;
    uint octree_depth;
} uniforms;

layout(std430, binding = 1) buffer octree {
    uint data[];
};

#define MAX_DEPTH 12
#define EPS 1e-5
const float infinity = 1. / 0.;


const uint SOLID = 256;
const uint EMPTY = 0;
const uint SUBVOXELS[8] = uint[](
    1, 2, 4, 8, 16, 32, 64, 128
);

struct Ray {
    vec3 origin;
    vec3 dir;
};

Ray generate_ray()  {
    float x_ratio = float(gl_FragCoord.x) / float(uniforms.width);
    float y_ratio = float(gl_FragCoord.y) / float(uniforms.height);
    float aspect = float(uniforms.width) / float(uniforms.height);
    float a = tan(uniforms.fov/2.0);
    float a2 = a/aspect;
    vec3 view_center = uniforms.view_pos + uniforms.view_dir;

    vec3 target = view_center + uniforms.view_right * a * (2 * x_ratio - 1) + uniforms.view_up * a2 * (2 * y_ratio - 1);

    return Ray(uniforms.view_pos, normalize(target-uniforms.view_pos));
}

struct StackNode {
    vec3 origin;
    uint index;
};

bool inside_voxel(vec3 point, vec3 origin, float size) {
    vec3 diff = abs(point-origin);
    return max(max(diff.x, diff.y), diff.z) <= size;
}

float dist(Ray r, vec3 origin, float size) {
    vec3 t0 = (origin-size-r.origin)/r.dir;
    t0 = vec3(t0.x < 0 ? infinity : t0.x, t0.y < 0 ? infinity : t0.y, t0.z < 0 ? infinity : t0.z);
    vec3 t1 = (origin+size-r.origin)/r.dir;
    t1 = vec3(t1.x < 0 ? infinity : t1.x, t1.y < 0 ? infinity : t1.y, t1.z < 0 ? infinity : t1.z);
    float t = min(t0.x, min(t0.y, min(t0.z, min(t1.x, min(t1.y, t1.z))))) + EPS;
    return t;
}


void main() {
    StackNode stack[MAX_DEPTH];
    uint current_stack = 0;
    stack[current_stack] = StackNode(vec3(0.0), 0);
    while(true) {
        
    }


    bool hit = false;
    Ray ray = generate_ray();
    float t = dist(ray, vec3(0.0), uniforms.octree_size);
    vec3 new_ray_origin = ray.origin + ray.dir*t;

    outColorDebug = vec4(t, 0.0, 0.0, 1.0);
    outColor = vec4(inside_voxel(new_ray_origin, vec3(0.0), uniforms.octree_size), 0.0, 0.0, 1.0);
}